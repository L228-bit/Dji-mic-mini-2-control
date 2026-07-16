#include "VoiceComfortDSP.h"

#include <algorithm>

namespace
{
constexpr double pi = 3.14159265358979323846;
}

void VoiceComfortDSP::prepare(double newSampleRate, int maxChannels)
{
    sampleRate = newSampleRate > 1000.0 ? newSampleRate : 48000.0;
    states.resize(static_cast<size_t>(std::max(1, maxChannels)));
    updateCoefficients();
    reset();
}

void VoiceComfortDSP::reset()
{
    for (auto& state : states)
        state.reset();

    meterInputDb.store(-80.0f);
    meterOutputDb.store(-80.0f);
    meterDeEssDb.store(0.0f);
}

void VoiceComfortDSP::setParameters(const Parameters& parameters)
{
    current = parameters;
    current.soften = clamp(current.soften, 0.0f, 1.0f);
    current.fullness = clamp(current.fullness, 0.0f, 1.0f);
    current.deEss = clamp(current.deEss, 0.0f, 1.0f);
    current.compression = clamp(current.compression, 0.0f, 1.0f);
    current.outputDb = clamp(current.outputDb, -12.0f, 6.0f);
    updateCoefficients();
}

VoiceComfortDSP::Meter VoiceComfortDSP::getMeter() const
{
    return { meterInputDb.load(), meterOutputDb.load(), meterDeEssDb.load() };
}

void VoiceComfortDSP::process(float* const* channels, int numChannels, int numSamples)
{
    if (channels == nullptr || numChannels <= 0 || numSamples <= 0)
        return;

    if (static_cast<int>(states.size()) < numChannels)
        states.resize(static_cast<size_t>(numChannels));

    double inputEnergy = 0.0;
    double outputEnergy = 0.0;
    float maxDeEssReduction = 0.0f;
    const float outputGain = gainFromDb(current.outputDb);
    const float drive = 1.0f + current.fullness * 0.42f;
    const float driveNormalisation = 1.0f / std::tanh(drive);
    const float compressorThresholdDb = -12.0f - current.compression * 12.0f;
    const float compressorRatio = 1.0f + current.compression * 4.0f;
    const float makeUpGain = gainFromDb(current.compression * 2.4f);
    const float deEssThresholdDb = -27.0f + (1.0f - current.deEss) * 13.0f;

    for (int channel = 0; channel < numChannels; ++channel)
    {
        auto* data = channels[channel];
        if (data == nullptr)
            continue;

        auto& state = states[static_cast<size_t>(channel)];

        for (int sampleIndex = 0; sampleIndex < numSamples; ++sampleIndex)
        {
            const float input = data[sampleIndex];
            inputEnergy += static_cast<double>(input) * input;

            if (current.bypass)
            {
                data[sampleIndex] = input;
                outputEnergy += static_cast<double>(input) * input;
                continue;
            }

            float value = state.highPass.process(input);
            value = state.bodyShelf.process(value);
            value = state.harshPeak.process(value);
            value = state.airShelf.process(value);

            state.deEssLow += deEssLowPassCoefficient * (value - state.deEssLow);
            const float highBand = value - state.deEssLow;
            const float highMagnitude = std::abs(highBand);
            const float deEssCoefficient = highMagnitude > state.deEssEnvelope
                ? deEssAttackCoefficient : deEssReleaseCoefficient;
            state.deEssEnvelope = deEssCoefficient * state.deEssEnvelope
                                + (1.0f - deEssCoefficient) * highMagnitude;

            const float highDb = dbFromGain(state.deEssEnvelope);
            const float requestedReductionDb = highDb > deEssThresholdDb
                ? std::min(9.0f * current.deEss, (highDb - deEssThresholdDb) * (0.32f + 0.55f * current.deEss))
                : 0.0f;
            const float highGain = gainFromDb(-requestedReductionDb);
            value = state.deEssLow + highBand * highGain;
            maxDeEssReduction = std::max(maxDeEssReduction, requestedReductionDb);

            const float magnitude = std::abs(value);
            const float compressorCoefficient = magnitude > state.compressorEnvelope
                ? compressorAttackCoefficient : compressorReleaseCoefficient;
            state.compressorEnvelope = compressorCoefficient * state.compressorEnvelope
                                     + (1.0f - compressorCoefficient) * magnitude;

            const float envelopeDb = dbFromGain(state.compressorEnvelope);
            float compressorGainDb = 0.0f;
            if (envelopeDb > compressorThresholdDb)
            {
                const float compressedDb = compressorThresholdDb
                                         + (envelopeDb - compressorThresholdDb) / compressorRatio;
                compressorGainDb = compressedDb - envelopeDb;
            }

            value *= gainFromDb(compressorGainDb) * makeUpGain;
            value = std::tanh(value * drive) * driveNormalisation;
            value *= outputGain;

            // A final soft ceiling protects ears and speakers from preset changes.
            if (std::abs(value) > 0.944f)
            {
                const float sign = value < 0.0f ? -1.0f : 1.0f;
                const float excess = std::abs(value) - 0.944f;
                value = sign * (0.944f + 0.055f * std::tanh(excess / 0.055f));
            }

            data[sampleIndex] = value;
            outputEnergy += static_cast<double>(value) * value;
        }
    }

    const float divisor = static_cast<float>(std::max(1, numChannels * numSamples));
    meterInputDb.store(dbFromGain(std::sqrt(static_cast<float>(inputEnergy) / divisor)));
    meterOutputDb.store(dbFromGain(std::sqrt(static_cast<float>(outputEnergy) / divisor)));
    meterDeEssDb.store(maxDeEssReduction);
}

void VoiceComfortDSP::updateCoefficients()
{
    const float soften = current.soften;
    const float fullness = current.fullness;

    for (auto& state : states)
    {
        state.highPass.setHighPass(sampleRate, 65.0, 0.707);
        state.bodyShelf.setLowShelf(sampleRate, 170.0, fullness * 4.2, 0.75);
        state.harshPeak.setPeak(sampleRate, 3150.0, 1.0 + soften * 0.65, -soften * 10.0);
        state.airShelf.setHighShelf(sampleRate, 6800.0, -soften * 4.2, 0.72);
    }

    deEssLowPassCoefficient = 1.0f - std::exp(-2.0f * static_cast<float>(pi) * 4700.0f / static_cast<float>(sampleRate));
    deEssAttackCoefficient = coefficientForMilliseconds(sampleRate, 1.5f);
    deEssReleaseCoefficient = coefficientForMilliseconds(sampleRate, 75.0f);
    compressorAttackCoefficient = coefficientForMilliseconds(sampleRate, 12.0f);
    compressorReleaseCoefficient = coefficientForMilliseconds(sampleRate, 145.0f);
}

void VoiceComfortDSP::ChannelState::reset()
{
    highPass.reset();
    bodyShelf.reset();
    harshPeak.reset();
    airShelf.reset();
    deEssLow = 0.0f;
    deEssEnvelope = 0.0f;
    compressorEnvelope = 0.0f;
}

void VoiceComfortDSP::Biquad::reset()
{
    z1 = 0.0;
    z2 = 0.0;
}

float VoiceComfortDSP::Biquad::process(float input)
{
    const double output = b0 * input + z1;
    z1 = b1 * input - a1 * output + z2;
    z2 = b2 * input - a2 * output;
    return static_cast<float>(output);
}

void VoiceComfortDSP::Biquad::setHighPass(double sr, double frequency, double q)
{
    const double omega = 2.0 * pi * frequency / sr;
    const double cosine = std::cos(omega);
    const double sine = std::sin(omega);
    const double alpha = sine / (2.0 * q);
    const double a0 = 1.0 + alpha;
    b0 = ((1.0 + cosine) * 0.5) / a0;
    b1 = -(1.0 + cosine) / a0;
    b2 = b0;
    a1 = (-2.0 * cosine) / a0;
    a2 = (1.0 - alpha) / a0;
}

void VoiceComfortDSP::Biquad::setPeak(double sr, double frequency, double q, double gainDb)
{
    const double a = std::pow(10.0, gainDb / 40.0);
    const double omega = 2.0 * pi * frequency / sr;
    const double cosine = std::cos(omega);
    const double alpha = std::sin(omega) / (2.0 * q);
    const double a0 = 1.0 + alpha / a;
    b0 = (1.0 + alpha * a) / a0;
    b1 = (-2.0 * cosine) / a0;
    b2 = (1.0 - alpha * a) / a0;
    a1 = (-2.0 * cosine) / a0;
    a2 = (1.0 - alpha / a) / a0;
}

void VoiceComfortDSP::Biquad::setLowShelf(double sr, double frequency, double gainDb, double slope)
{
    const double a = std::pow(10.0, gainDb / 40.0);
    const double omega = 2.0 * pi * frequency / sr;
    const double cosine = std::cos(omega);
    const double sine = std::sin(omega);
    const double alpha = sine * 0.5 * std::sqrt((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0);
    const double beta = 2.0 * std::sqrt(a) * alpha;
    const double a0 = (a + 1.0) + (a - 1.0) * cosine + beta;
    b0 = a * ((a + 1.0) - (a - 1.0) * cosine + beta) / a0;
    b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cosine) / a0;
    b2 = a * ((a + 1.0) - (a - 1.0) * cosine - beta) / a0;
    a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cosine) / a0;
    a2 = ((a + 1.0) + (a - 1.0) * cosine - beta) / a0;
}

void VoiceComfortDSP::Biquad::setHighShelf(double sr, double frequency, double gainDb, double slope)
{
    const double a = std::pow(10.0, gainDb / 40.0);
    const double omega = 2.0 * pi * frequency / sr;
    const double cosine = std::cos(omega);
    const double sine = std::sin(omega);
    const double alpha = sine * 0.5 * std::sqrt((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0);
    const double beta = 2.0 * std::sqrt(a) * alpha;
    const double a0 = (a + 1.0) - (a - 1.0) * cosine + beta;
    b0 = a * ((a + 1.0) + (a - 1.0) * cosine + beta) / a0;
    b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cosine) / a0;
    b2 = a * ((a + 1.0) + (a - 1.0) * cosine - beta) / a0;
    a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cosine) / a0;
    a2 = ((a + 1.0) - (a - 1.0) * cosine - beta) / a0;
}

float VoiceComfortDSP::coefficientForMilliseconds(double sr, float milliseconds)
{
    return std::exp(-1.0f / (0.001f * milliseconds * static_cast<float>(sr)));
}

float VoiceComfortDSP::gainFromDb(float db)
{
    return std::pow(10.0f, db / 20.0f);
}

float VoiceComfortDSP::dbFromGain(float gain)
{
    return 20.0f * std::log10(std::max(gain, 1.0e-5f));
}

float VoiceComfortDSP::clamp(float value, float low, float high)
{
    return std::max(low, std::min(high, value));
}

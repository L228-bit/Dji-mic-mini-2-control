#pragma once

#include <array>
#include <atomic>
#include <cmath>
#include <vector>

class VoiceComfortDSP
{
public:
    struct Parameters
    {
        float soften = 0.65f;
        float fullness = 0.55f;
        float deEss = 0.55f;
        float compression = 0.55f;
        float outputDb = 0.0f;
        bool bypass = false;
    };

    struct Meter
    {
        float inputDb = -80.0f;
        float outputDb = -80.0f;
        float deEssReductionDb = 0.0f;
    };

    void prepare(double sampleRate, int maxChannels);
    void reset();
    void setParameters(const Parameters& parameters);
    void process(float* const* channels, int numChannels, int numSamples);
    Meter getMeter() const;

private:
    struct Biquad
    {
        double b0 = 1.0, b1 = 0.0, b2 = 0.0, a1 = 0.0, a2 = 0.0;
        double z1 = 0.0, z2 = 0.0;

        void reset();
        void setHighPass(double sampleRate, double frequencyHz, double q);
        void setLowShelf(double sampleRate, double frequencyHz, double gainDb, double slope);
        void setHighShelf(double sampleRate, double frequencyHz, double gainDb, double slope);
        void setPeak(double sampleRate, double frequencyHz, double q, double gainDb);
        float process(float sample);
    };

    struct ChannelState
    {
        Biquad highPass;
        Biquad bodyShelf;
        Biquad harshPeak;
        Biquad airShelf;
        float deEssLow = 0.0f;
        float deEssEnvelope = 0.0f;
        float compressorEnvelope = 0.0f;

        void reset();
    };

    double sampleRate = 48000.0;
    Parameters current;
    std::vector<ChannelState> states;
    float deEssLowPassCoefficient = 0.0f;
    float deEssAttackCoefficient = 0.0f;
    float deEssReleaseCoefficient = 0.0f;
    float compressorAttackCoefficient = 0.0f;
    float compressorReleaseCoefficient = 0.0f;

    std::atomic<float> meterInputDb { -80.0f };
    std::atomic<float> meterOutputDb { -80.0f };
    std::atomic<float> meterDeEssDb { 0.0f };

    void updateCoefficients();
    static float coefficientForMilliseconds(double sampleRate, float milliseconds);
    static float gainFromDb(float db);
    static float dbFromGain(float gain);
    static float clamp(float value, float low, float high);
};

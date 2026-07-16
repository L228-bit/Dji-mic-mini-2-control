#include <juce_audio_utils/juce_audio_utils.h>

#include <atomic>
#include <chrono>
#include <cstring>
#include <iostream>
#include <mutex>
#include <string>
#include <thread>

#include "SystemAudioRouter.h"
#include "ReceiverShortcut.h"
#include "VoiceComfortDSP.h"

namespace
{
class ComfortEngine final : private juce::AudioIODeviceCallback
{
public:
    ~ComfortEngine() override { stop(); }

    bool start()
    {
        if (running.load())
            return true;

        auto error = deviceManager.initialise(2, 2, nullptr, true);
        if (error.isNotEmpty())
        {
            setError(error.toStdString());
            return false;
        }

        selectPreferredDevices();
        if (deviceManager.getCurrentAudioDevice() == nullptr)
        {
            setError("无法打开音频输入或输出设备");
            deviceManager.closeAudioDevice();
            return false;
        }

        deviceManager.addAudioCallback(this);
        if (!router.routeSystemOutputToBlackHole())
        {
            deviceManager.removeAudioCallback(this);
            deviceManager.closeAudioDevice();
            setError(router.getLastError());
            return false;
        }

        setError({});
        running.store(true);
        return true;
    }

    void stop()
    {
        if (!running.exchange(false))
            return;
        deviceManager.removeAudioCallback(this);
        deviceManager.closeAudioDevice();
        router.restorePreviousOutput();
        dsp.reset();
    }

    void setParameters(const juce::DynamicObject& object)
    {
        setFloat(object, "soften", soften, 0.0f, 1.0f);
        setFloat(object, "fullness", fullness, 0.0f, 1.0f);
        setFloat(object, "deEss", deEss, 0.0f, 1.0f);
        setFloat(object, "compression", compression, 0.0f, 1.0f);
        setFloat(object, "outputDb", outputDb, -12.0f, 6.0f);
        if (object.hasProperty("bypass"))
            bypass.store(static_cast<bool>(object.getProperty("bypass")));
    }

    bool startReceiverShortcut() { return receiverShortcut.start(); }
    void stopReceiverShortcut() { receiverShortcut.stop(); }

    juce::var status() const
    {
        auto result = std::make_unique<juce::DynamicObject>();
        const auto meter = dsp.getMeter();
        result->setProperty("running", running.load());
        result->setProperty("inputDb", meter.inputDb);
        result->setProperty("outputDb", meter.outputDb);
        result->setProperty("deEssReductionDb", meter.deEssReductionDb);
        result->setProperty("soften", soften.load());
        result->setProperty("fullness", fullness.load());
        result->setProperty("deEss", deEss.load());
        result->setProperty("compression", compression.load());
        result->setProperty("gainDb", outputDb.load());
        result->setProperty("bypass", bypass.load());
        result->setProperty("shortcutAvailable", true);
        result->setProperty("shortcutRunning", receiverShortcut.isRunning());
        result->setProperty("shortcutError", receiverShortcut.getError());
        {
            const std::lock_guard lock(errorMutex);
            result->setProperty("error", lastError);
        }
        return juce::var(result.release());
    }

private:
    juce::AudioDeviceManager deviceManager;
    SystemAudioRouter router;
    ReceiverShortcut receiverShortcut;
    VoiceComfortDSP dsp;
    std::atomic<bool> running { false };
    std::atomic<float> soften { 0.78f };
    std::atomic<float> fullness { 0.53f };
    std::atomic<float> deEss { 0.68f };
    std::atomic<float> compression { 0.58f };
    std::atomic<float> outputDb { -0.5f };
    std::atomic<bool> bypass { false };
    mutable std::mutex errorMutex;
    juce::String lastError;

    static void setFloat(const juce::DynamicObject& object, const juce::Identifier& key,
                         std::atomic<float>& destination, float low, float high)
    {
        if (object.hasProperty(key))
            destination.store(juce::jlimit(low, high, static_cast<float>(object.getProperty(key))));
    }

    void setError(const std::string& error)
    {
        const std::lock_guard lock(errorMutex);
        lastError = juce::String::fromUTF8(error.c_str());
    }

    void selectPreferredDevices()
    {
        auto* type = deviceManager.getCurrentDeviceTypeObject();
        if (type == nullptr)
            return;
        type->scanForDevices();
        auto setup = deviceManager.getAudioDeviceSetup();

        for (const auto& name : type->getDeviceNames(true))
            if (name.containsIgnoreCase("BlackHole"))
            {
                setup.inputDeviceName = name;
                break;
            }

        juce::String output;
        const auto outputs = type->getDeviceNames(false);
        for (const auto& preferred : { "MacBook Pro", "Speakers", "扬声器", "Built-in Output" })
        {
            for (const auto& name : outputs)
                if (!name.containsIgnoreCase("BlackHole") && name.containsIgnoreCase(preferred))
                {
                    output = name;
                    break;
                }
            if (output.isNotEmpty())
                break;
        }
        if (output.isEmpty())
            for (const auto& name : outputs)
                if (!name.containsIgnoreCase("BlackHole"))
                {
                    output = name;
                    break;
                }
        if (output.isNotEmpty())
            setup.outputDeviceName = output;

        setup.inputChannels.setRange(0, 2, true);
        setup.outputChannels.setRange(0, 2, true);
        setup.useDefaultInputChannels = false;
        setup.useDefaultOutputChannels = false;
        const auto error = deviceManager.setAudioDeviceSetup(setup, true);
        if (error.isNotEmpty())
            setError(error.toStdString());
    }

    void audioDeviceAboutToStart(juce::AudioIODevice* device) override
    {
        dsp.prepare(device != nullptr ? device->getCurrentSampleRate() : 48000.0, 2);
    }

    void audioDeviceStopped() override { dsp.reset(); }

    void audioDeviceIOCallbackWithContext(const float* const* inputs, int inputChannels,
                                          float* const* outputs, int outputChannels,
                                          int samples,
                                          const juce::AudioIODeviceCallbackContext&) override
    {
        for (int channel = 0; channel < outputChannels; ++channel)
        {
            if (outputs[channel] == nullptr)
                continue;
            const auto source = inputChannels > 0 ? juce::jmin(channel, inputChannels - 1) : -1;
            if (source >= 0 && inputs[source] != nullptr)
                std::memcpy(outputs[channel], inputs[source], static_cast<size_t>(samples) * sizeof(float));
            else
                juce::FloatVectorOperations::clear(outputs[channel], samples);
        }

        VoiceComfortDSP::Parameters parameters;
        parameters.soften = soften.load();
        parameters.fullness = fullness.load();
        parameters.deEss = deEss.load();
        parameters.compression = compression.load();
        parameters.outputDb = outputDb.load();
        parameters.bypass = bypass.load();
        dsp.setParameters(parameters);
        dsp.process(outputs, juce::jmin(2, outputChannels), samples);
    }
};
}

int main()
{
    juce::ScopedJuceInitialiser_GUI juceInitialiser;
    ComfortEngine engine;
    std::atomic<bool> alive { true };
    std::mutex outputMutex;

    std::thread reporter([&]
    {
        while (alive.load())
        {
            {
                const std::lock_guard lock(outputMutex);
                std::cout << juce::JSON::toString(engine.status(), true).toStdString() << std::endl;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(100));
        }
    });

    std::string line;
    while (std::getline(std::cin, line))
    {
        const auto parsed = juce::JSON::parse(juce::String::fromUTF8(line.c_str()));
        auto* object = parsed.getDynamicObject();
        if (object == nullptr)
            continue;
        const auto action = object->getProperty("action").toString();
        if (action == "start")
            engine.start();
        else if (action == "stop")
            engine.stop();
        else if (action == "set")
            engine.setParameters(*object);
        else if (action == "shortcut_start")
            engine.startReceiverShortcut();
        else if (action == "shortcut_stop")
            engine.stopReceiverShortcut();
        else if (action == "quit")
            break;
    }

    engine.stopReceiverShortcut();
    engine.stop();
    alive.store(false);
    reporter.join();
    return 0;
}

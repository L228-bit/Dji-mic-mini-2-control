#pragma once

#include <CoreAudio/CoreAudio.h>

#include <string>

class SystemAudioRouter
{
public:
    bool routeSystemOutputToBlackHole();
    void restorePreviousOutput();
    const std::string& getLastError() const { return lastError; }

private:
    AudioDeviceID previousDefaultOutput = kAudioObjectUnknown;
    AudioDeviceID previousSystemOutput = kAudioObjectUnknown;
    bool routingActive = false;
    std::string lastError;

    static AudioDeviceID getDefaultDevice(AudioObjectPropertySelector selector);
    static bool setDefaultDevice(AudioObjectPropertySelector selector, AudioDeviceID device);
    static AudioDeviceID findOutputDevice(const std::string& requiredText, bool physicalOnly);
    static AudioDeviceID findPreferredPhysicalOutput();
    static std::string getDeviceName(AudioDeviceID device);
    static bool hasOutputChannels(AudioDeviceID device);
    static bool isDeviceAvailable(AudioDeviceID device);
};

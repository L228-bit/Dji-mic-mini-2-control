#include "SystemAudioRouter.h"

#include <CoreFoundation/CoreFoundation.h>

#include <algorithm>
#include <cctype>
#include <vector>

namespace
{
std::string lowerCase(std::string value)
{
    std::transform(value.begin(), value.end(), value.begin(),
                   [](unsigned char character) { return static_cast<char>(std::tolower(character)); });
    return value;
}
bool contains(const std::string& value, const std::string& query)
{
    return lowerCase(value).find(lowerCase(query)) != std::string::npos;
}
}

bool SystemAudioRouter::routeSystemOutputToBlackHole()
{
    lastError.clear();
    previousDefaultOutput = getDefaultDevice(kAudioHardwarePropertyDefaultOutputDevice);
    previousSystemOutput = getDefaultDevice(kAudioHardwarePropertyDefaultSystemOutputDevice);

    const auto blackHole = findOutputDevice("BlackHole", false);
    if (blackHole == kAudioObjectUnknown)
    {
        lastError = "BlackHole 2ch output device was not found";
        return false;
    }

    if (previousDefaultOutput == blackHole)
        previousDefaultOutput = findPreferredPhysicalOutput();
    if (previousSystemOutput == blackHole)
        previousSystemOutput = previousDefaultOutput;

    if (!setDefaultDevice(kAudioHardwarePropertyDefaultOutputDevice, blackHole))
    {
        lastError = "Could not switch the default output to BlackHole 2ch";
        return false;
    }

    // System sounds use a separate CoreAudio default. Keep both paths consistent.
    setDefaultDevice(kAudioHardwarePropertyDefaultSystemOutputDevice, blackHole);
    routingActive = true;
    return true;
}

void SystemAudioRouter::restorePreviousOutput()
{
    if (!routingActive)
        return;

    auto output = previousDefaultOutput;
    if (!isDeviceAvailable(output) || contains(getDeviceName(output), "BlackHole"))
        output = findPreferredPhysicalOutput();

    auto systemOutput = previousSystemOutput;
    if (!isDeviceAvailable(systemOutput) || contains(getDeviceName(systemOutput), "BlackHole"))
        systemOutput = output;

    if (output != kAudioObjectUnknown)
        setDefaultDevice(kAudioHardwarePropertyDefaultOutputDevice, output);
    if (systemOutput != kAudioObjectUnknown)
        setDefaultDevice(kAudioHardwarePropertyDefaultSystemOutputDevice, systemOutput);

    routingActive = false;
}

AudioDeviceID SystemAudioRouter::getDefaultDevice(AudioObjectPropertySelector selector)
{
    AudioObjectPropertyAddress address { selector,
                                         kAudioObjectPropertyScopeGlobal,
                                         kAudioObjectPropertyElementMain };
    AudioDeviceID device = kAudioObjectUnknown;
    UInt32 size = sizeof(device);
    return AudioObjectGetPropertyData(kAudioObjectSystemObject, &address, 0, nullptr,
                                      &size, &device) == noErr
        ? device : kAudioObjectUnknown;
}

bool SystemAudioRouter::setDefaultDevice(AudioObjectPropertySelector selector, AudioDeviceID device)
{
    if (device == kAudioObjectUnknown)
        return false;

    AudioObjectPropertyAddress address { selector,
                                         kAudioObjectPropertyScopeGlobal,
                                         kAudioObjectPropertyElementMain };
    return AudioObjectSetPropertyData(kAudioObjectSystemObject, &address, 0, nullptr,
                                      sizeof(device), &device) == noErr;
}

AudioDeviceID SystemAudioRouter::findOutputDevice(const std::string& requiredText, bool physicalOnly)
{
    AudioObjectPropertyAddress address { kAudioHardwarePropertyDevices,
                                         kAudioObjectPropertyScopeGlobal,
                                         kAudioObjectPropertyElementMain };
    UInt32 size = 0;
    if (AudioObjectGetPropertyDataSize(kAudioObjectSystemObject, &address, 0, nullptr, &size) != noErr)
        return kAudioObjectUnknown;

    std::vector<AudioDeviceID> devices(size / sizeof(AudioDeviceID));
    if (AudioObjectGetPropertyData(kAudioObjectSystemObject, &address, 0, nullptr,
                                   &size, devices.data()) != noErr)
        return kAudioObjectUnknown;

    for (const auto device : devices)
    {
        if (!hasOutputChannels(device))
            continue;

        const auto name = getDeviceName(device);
        if (physicalOnly && contains(name, "BlackHole"))
            continue;
        if (requiredText.empty() || contains(name, requiredText))
            return device;
    }

    return kAudioObjectUnknown;
}

AudioDeviceID SystemAudioRouter::findPreferredPhysicalOutput()
{
    for (const auto& preferred : { "MacBook Pro", "Speakers", "扬声器", "Built-in Output" })
        if (const auto device = findOutputDevice(preferred, true); device != kAudioObjectUnknown)
            return device;

    return findOutputDevice({}, true);
}

std::string SystemAudioRouter::getDeviceName(AudioDeviceID device)
{
    if (device == kAudioObjectUnknown)
        return {};

    AudioObjectPropertyAddress address { kAudioObjectPropertyName,
                                         kAudioObjectPropertyScopeGlobal,
                                         kAudioObjectPropertyElementMain };
    CFStringRef name = nullptr;
    UInt32 size = sizeof(name);
    if (AudioObjectGetPropertyData(device, &address, 0, nullptr, &size, &name) != noErr
        || name == nullptr)
        return {};

    char buffer[512] {};
    const bool converted = CFStringGetCString(name, buffer, sizeof(buffer), kCFStringEncodingUTF8);
    CFRelease(name);
    return converted ? std::string(buffer) : std::string();
}

bool SystemAudioRouter::hasOutputChannels(AudioDeviceID device)
{
    AudioObjectPropertyAddress address { kAudioDevicePropertyStreamConfiguration,
                                         kAudioDevicePropertyScopeOutput,
                                         kAudioObjectPropertyElementMain };
    UInt32 size = 0;
    if (AudioObjectGetPropertyDataSize(device, &address, 0, nullptr, &size) != noErr || size == 0)
        return false;

    std::vector<unsigned char> storage(size);
    auto* list = reinterpret_cast<AudioBufferList*>(storage.data());
    if (AudioObjectGetPropertyData(device, &address, 0, nullptr, &size, list) != noErr)
        return false;

    for (UInt32 index = 0; index < list->mNumberBuffers; ++index)
        if (list->mBuffers[index].mNumberChannels > 0)
            return true;
    return false;
}

bool SystemAudioRouter::isDeviceAvailable(AudioDeviceID device)
{
    if (device == kAudioObjectUnknown)
        return false;
    AudioObjectPropertyAddress address { kAudioObjectPropertyName,
                                         kAudioObjectPropertyScopeGlobal,
                                         kAudioObjectPropertyElementMain };
    return AudioObjectHasProperty(device, &address);
}

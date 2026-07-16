#pragma once

#include <ApplicationServices/ApplicationServices.h>

#include <atomic>
#include <condition_variable>
#include <mutex>
#include <thread>

#include <juce_core/juce_core.h>

class ReceiverShortcut
{
public:
    ReceiverShortcut() = default;
    ~ReceiverShortcut();

    bool start();
    void stop();
    bool isRunning() const { return running.load(); }
    juce::String getError() const;

private:
    std::atomic<bool> running { false };
    std::atomic<int64_t> lastTriggerMs { 0 };
    mutable std::mutex stateMutex;
    std::condition_variable stateReady;
    bool initialised = false;
    bool initialisationSucceeded = false;
    juce::String lastError;
    CFMachPortRef eventTap = nullptr;
    CFRunLoopSourceRef runLoopSource = nullptr;
    CFRunLoopRef runLoop = nullptr;
    std::thread eventThread;

    static CGEventRef eventCallback(CGEventTapProxy proxy, CGEventType type,
                                    CGEventRef event, void* context);
    CGEventRef handleEvent(CGEventType type, CGEventRef event);
    bool applyMapping(bool enabled);
    void setError(const juce::String& error);
};

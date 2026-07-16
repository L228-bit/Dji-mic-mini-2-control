#include "ReceiverShortcut.h"

#include <chrono>

namespace
{
constexpr CGKeyCode f20KeyCode = 90;

void postKey(CGKeyCode code, CGEventFlags flags, bool down)
{
    if (const auto event = CGEventCreateKeyboardEvent(nullptr, code, down))
    {
        CGEventSetFlags(event, flags);
        CGEventPost(kCGHIDEventTap, event);
        CFRelease(event);
    }
}

void postFnControlTap()
{
    constexpr CGKeyCode fnKey = 63;
    constexpr CGKeyCode controlKey = 59;
    constexpr auto fn = kCGEventFlagMaskSecondaryFn;
    constexpr auto both = static_cast<CGEventFlags>(kCGEventFlagMaskSecondaryFn
                                                     | kCGEventFlagMaskControl);
    postKey(fnKey, fn, true);
    postKey(controlKey, both, true);
    juce::Thread::sleep(45);
    postKey(controlKey, fn, false);
    postKey(fnKey, 0, false);
}
}

ReceiverShortcut::~ReceiverShortcut()
{
    stop();
}

bool ReceiverShortcut::start()
{
    if (running.load())
        return true;
    if (!applyMapping(true))
        return false;

    {
        const std::lock_guard lock(stateMutex);
        initialised = false;
        initialisationSucceeded = false;
    }
    eventThread = std::thread([this]
    {
        const auto mask = CGEventMaskBit(kCGEventKeyDown) | CGEventMaskBit(kCGEventKeyUp);
        const auto tap = CGEventTapCreate(kCGSessionEventTap, kCGHeadInsertEventTap,
                                          kCGEventTapOptionDefault, mask,
                                          eventCallback, this);
        if (tap == nullptr)
        {
            setError("请允许本应用的辅助功能与输入监控权限");
            const std::lock_guard lock(stateMutex);
            initialised = true;
            initialisationSucceeded = false;
            stateReady.notify_all();
            return;
        }

        const auto source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0);
        const auto loop = CFRunLoopGetCurrent();
        CFRetain(loop);
        {
            const std::lock_guard lock(stateMutex);
            eventTap = tap;
            runLoopSource = source;
            runLoop = loop;
            initialised = true;
            initialisationSucceeded = source != nullptr;
        }
        stateReady.notify_all();

        if (source != nullptr)
        {
            CFRunLoopAddSource(loop, source, kCFRunLoopCommonModes);
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wnullable-to-nonnull-conversion"
            CGEventTapEnable(tap, true);
#pragma clang diagnostic pop
            running.store(true);
            CFRunLoopRun();
            running.store(false);
            CFRunLoopRemoveSource(loop, source, kCFRunLoopCommonModes);
        }

        {
            const std::lock_guard lock(stateMutex);
            eventTap = nullptr;
            runLoopSource = nullptr;
            runLoop = nullptr;
        }
        if (source != nullptr)
            CFRelease(source);
        CFRelease(tap);
        CFRelease(loop);
    });

    std::unique_lock lock(stateMutex);
    stateReady.wait_for(lock, std::chrono::seconds(2), [this] { return initialised; });
    const auto okay = initialised && initialisationSucceeded;
    lock.unlock();
    if (!okay)
    {
        if (eventThread.joinable())
            eventThread.join();
        applyMapping(false);
    }
    return okay;
}

void ReceiverShortcut::stop()
{
    CFRunLoopRef loop = nullptr;
    {
        const std::lock_guard lock(stateMutex);
        loop = runLoop;
        if (loop != nullptr)
            CFRetain(loop);
    }
    if (loop != nullptr)
    {
        CFRunLoopStop(loop);
        CFRelease(loop);
    }
    if (eventThread.joinable())
        eventThread.join();
    running.store(false);
    applyMapping(false);
}

juce::String ReceiverShortcut::getError() const
{
    const std::lock_guard lock(stateMutex);
    return lastError;
}

CGEventRef ReceiverShortcut::eventCallback(CGEventTapProxy, CGEventType type,
                                           CGEventRef event, void* context)
{
    return static_cast<ReceiverShortcut*>(context)->handleEvent(type, event);
}

CGEventRef ReceiverShortcut::handleEvent(CGEventType type, CGEventRef event)
{
    if (type == kCGEventTapDisabledByTimeout || type == kCGEventTapDisabledByUserInput)
    {
        const std::lock_guard lock(stateMutex);
        if (eventTap != nullptr)
            CGEventTapEnable(eventTap, true);
        return event;
    }
    if (CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode) != f20KeyCode)
        return event;

    if (type == kCGEventKeyDown && !CGEventGetIntegerValueField(event, kCGKeyboardEventAutorepeat))
    {
        const auto now = static_cast<int64_t>(juce::Time::getMillisecondCounterHiRes());
        const auto previous = lastTriggerMs.exchange(now);
        if (now - previous > 180)
            postFnControlTap();
    }
    return nullptr;
}

bool ReceiverShortcut::applyMapping(bool enabled)
{
    const auto mapping = enabled
        ? R"({"UserKeyMapping":[{"HIDKeyboardModifierMappingSrc":0x0000000C000000E9,"HIDKeyboardModifierMappingDst":0x000000070000006F},{"HIDKeyboardModifierMappingSrc":0x0000000C000000EA,"HIDKeyboardModifierMappingDst":0x000000070000006F}]})"
        : R"({"UserKeyMapping":[]})";
    juce::StringArray command {
        "/usr/bin/hidutil", "property", "--matching",
        R"({"VendorID":0x2ca3,"ProductID":0x4011})", "--set", mapping
    };
    juce::ChildProcess process;
    if (!process.start(command) || !process.waitForProcessToFinish(2000) || process.getExitCode() != 0)
    {
        setError("无法设置 DJI 接收器快捷键映射");
        return false;
    }
    if (enabled)
        setError({});
    return true;
}

void ReceiverShortcut::setError(const juce::String& error)
{
    const std::lock_guard lock(stateMutex);
    lastError = error;
}

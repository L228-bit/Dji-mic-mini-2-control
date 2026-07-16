cmake_minimum_required(VERSION 3.22)

project(VoiceComfortEngine VERSION 1.2.1 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT DEFINED JUCE_DIR)
    message(FATAL_ERROR "Set JUCE_DIR to a local JUCE checkout.")
endif()

add_subdirectory("${JUCE_DIR}" JUCE)

juce_add_console_app(VoiceComfortEngine PRODUCT_NAME "Voice Comfort Engine")

target_sources(VoiceComfortEngine PRIVATE
    Source/EngineMain.cpp
    Source/ReceiverShortcut.cpp
    Source/ReceiverShortcut.h
    Source/VoiceComfortDSP.cpp
    Source/VoiceComfortDSP.h
    Source/SystemAudioRouter.cpp
    Source/SystemAudioRouter.h)

target_compile_definitions(VoiceComfortEngine PRIVATE
    JUCE_WEB_BROWSER=0
    JUCE_USE_CURL=0)

target_link_libraries(VoiceComfortEngine PRIVATE
    juce::juce_audio_utils
    "-framework ApplicationServices"
    PUBLIC
    juce::juce_recommended_config_flags
    juce::juce_recommended_lto_flags
    juce::juce_recommended_warning_flags)

#include "pch.h"
#include "App.xaml.h"

int WINAPI WinMain(HINSTANCE, HINSTANCE, LPSTR, int) {
    winrt::init_apartment(winrt::apartment_type::single_threaded);

    ::SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

    winrt::Microsoft::UI::Xaml::Application::Start([](auto&&) {
        winrt::make<winrt::NotificationManager::implementation::App>();
    });

    return 0;
}

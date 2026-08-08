#pragma once
#include "App.xaml.g.h"

namespace winrt::NotificationManager::implementation {
    struct App : AppT<App> {
        App();
        void OnLaunched(winrt::Microsoft::UI::Xaml::LaunchActivatedEventArgs const&);
    private:
        winrt::Microsoft::UI::Xaml::Window _mainWindow{ nullptr };
    };
}

namespace winrt::NotificationManager::factory_implementation {
    struct App : AppT<App, implementation::App> {};
}

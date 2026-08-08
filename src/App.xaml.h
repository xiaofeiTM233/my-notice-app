#pragma once
#include "pch.h"
#include "App.g.h"

namespace winrt::WinNotify::implementation
{
    struct App : AppT<App>
    {
        App();
        void OnLaunched(Microsoft::UI::Xaml::LaunchActivatedEventArgs const&);
    };
}

namespace winrt::WinNotify::factory_implementation
{
    struct App : AppT<App, implementation::App> {};
}

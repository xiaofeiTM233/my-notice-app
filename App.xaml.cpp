#include "pch.h"
#include "App.xaml.h"
#include "MainWindow.xaml.h"
#include "Services/NotificationService.h"
#include "Helpers/TrayIcon.h"
#include "Models/AppConfig.h"

using namespace winrt;
using namespace Microsoft::UI::Xaml;

namespace winrt::NotificationManager::implementation {

App::App() {
    InitializeComponent();
}

void App::OnLaunched(LaunchActivatedEventArgs const&) {
    auto cfg = nm::AppConfig::Load("config.json");

    _mainWindow = make<MainWindow>();
    _mainWindow.Activate();

    if (cfg.startMinimized) {
        // Will be handled by MainWindow after initialization
    }
}

} // namespace winrt::NotificationManager::implementation

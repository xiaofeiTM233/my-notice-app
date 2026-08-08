#include "pch.h"
#include "App.xaml.h"
#include "MainWindow.xaml.h"
#include "services/ConfigManager.h"
#include "services/NotificationService.h"
#include "ui/PopupManager.h"

#if __has_include("App.g.cpp")
#include "App.g.cpp"
#endif

using namespace winrt;
using namespace Microsoft::UI::Xaml;

namespace winrt::WinNotify::implementation
{
    App::App()
    {
        InitializeComponent();

        // 加载配置
        std::wstring configPath = L"config.json";
        ConfigManager::Instance().Load(configPath);

        // 初始化通知服务
        NotificationService::Instance().Init();

        // 初始化弹窗管理器
        PopupManager::Instance().Init();

        // 绑定通知到弹窗
        NotificationService::Instance().SetOnPopup([](const Notification& n) {
            PopupManager::Instance().ShowNotification(n);
        });
    }

    void App::OnLaunched(LaunchActivatedEventArgs const&)
    {
        auto window = make<MainWindow>();
        window.Activate();

        auto& cfg = ConfigManager::Instance().Get();
        if (cfg.startMinimized)
        {
            // 最小化到托盘
            // window.Minimize(); // 需要在 MainWindow 中实现
        }
    }
}

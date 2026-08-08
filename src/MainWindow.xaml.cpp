#include "pch.h"
#include "MainWindow.xaml.h"
#include "services/NotificationService.h"
#include "services/ConfigManager.h"
#include "ui/PopupManager.h"
#if __has_include("MainWindow.g.cpp")
#include "MainWindow.g.cpp"
#endif

using namespace winrt;
using namespace Microsoft::UI::Xaml;

namespace winrt::WinNotify::implementation
{
    MainWindow::MainWindow()
    {
        InitializeComponent();

        // 获取窗口句柄
        auto native = this->try_as<IWindowNative>();
        if (native) native->get_WindowHandle(&m_hwnd);

        // 设置窗口大小
        if (m_hwnd)
        {
            SetWindowPos(m_hwnd, nullptr, 0, 0, 420, 680, SWP_NOMOVE | SWP_NOZORDER);
            // 居中
            RECT rc;
            GetWindowRect(m_hwnd, &rc);
            int w = rc.right - rc.left;
            int h = rc.bottom - rc.top;
            int x = (GetSystemMetrics(SM_CXSCREEN) - w) / 2;
            int y = (GetSystemMetrics(SM_CYSCREEN) - h) / 2;
            SetWindowPos(m_hwnd, nullptr, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
        }

        SetupTitleBar();
        SetupTrayIcon();

        // 监听通知列表变化
        NotificationService::Instance().SetOnListChanged([this]() {
            auto disp = DispatcherQueue();
            disp.TryEnqueue([this]() { NotifyCenter().RefreshList(); });
        });

        NotificationService::Instance().SetOnUnreadChanged([this]() {
            auto disp = DispatcherQueue();
            disp.TryEnqueue([this]() { NotifyCenter().UpdateUnreadBadge(); });
        });

        // 连接状态
        NotificationService::Instance().Backend().SetOnStatus([this](bool connected, const std::wstring& msg) {
            auto disp = DispatcherQueue();
            disp.TryEnqueue([this, connected, msg]() {
                NotifyCenter().UpdateConnStatus(connected, msg);
            });
        });
    }

    MainWindow::~MainWindow()
    {
        NotificationService::Instance().Shutdown();
    }

    void MainWindow::SetupTitleBar()
    {
        ExtendsContentIntoTitleBar(true);
        SetTitleBar(AppTitleBar());

        // 自定义标题栏按钮悬停效果
        auto addHover = [](Button& btn) {
            btn.PointerEntered([&btn](auto&&, auto&&) {
                btn.Background(SolidColorBrush(Windows::UI::ColorHelper::FromArgb(25, 255, 255, 255)));
            });
            btn.PointerExited([&btn](auto&&, auto&&) {
                btn.Background(SolidColorBrush(Windows::UI::Colors::Transparent()));
            });
        };
        addHover(MinimizeBtn());
        addHover(CloseBtn());

        CloseBtn().PointerEntered([this](auto&&, auto&&) {
            CloseBtn().Background(SolidColorBrush(Windows::UI::ColorHelper::FromArgb(255, 232, 17, 35)));
            CloseBtn().FindName(L"").as<FontIcon>();
        });
    }

    void MainWindow::SetupTrayIcon()
    {
        if (!m_hwnd) return;

        // 加载默认图标
        HICON hIcon = LoadIcon(nullptr, IDI_INFORMATION);

        m_trayIcon = std::make_unique<TrayIcon>();
        m_trayIcon->Create(m_hwnd, hIcon, L"WinNotify 通知管理器");
        m_trayIcon->SetMenuCallback([this](int cmd) { OnTrayMenuCmd(cmd); });

        // 子类化窗口过程以处理托盘消息
        m_origWndProc = (WNDPROC)SetWindowLongPtr(m_hwnd, GWLP_WNDPROC, (LONG_PTR)WndProc);
        SetWindowLongPtr(m_hwnd, GWLP_USERDATA, (LONG_PTR)this);
    }

    LRESULT CALLBACK MainWindow::WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam)
    {
        auto* self = (MainWindow*)GetWindowLongPtr(hwnd, GWLP_USERDATA);
        if (self && msg == TrayIcon::WM_TRAYMSG)
        {
            self->OnTrayMessage(wParam, lParam);
            return 0;
        }
        if (self && msg == WM_COMMAND)
        {
            if (LOWORD(wParam) >= 1001 && LOWORD(wParam) <= 1010)
            {
                self->OnTrayMenuCmd(LOWORD(wParam));
                return 0;
            }
        }
        if (self && msg == WM_CLOSE)
        {
            auto& cfg = ConfigManager::Instance().Get();
            if (cfg.closeToTray)
            {
                self->HideToTray();
                return 0;
            }
        }
        if (self && self->m_origWndProc)
            return CallWindowProc(self->m_origWndProc, hwnd, msg, wParam, lParam);
        return DefWindowProc(hwnd, msg, wParam, lParam);
    }

    void MainWindow::OnTrayMessage(WPARAM, LPARAM lParam)
    {
        if (LOWORD(lParam) == WM_RBUTTONUP || LOWORD(lParam) == WM_CONTEXTMENU)
        {
            POINT pt;
            GetCursorPos(&pt);
            m_trayIcon->ShowContextMenu(pt.x, pt.y);
        }
        else if (LOWORD(lParam) == WM_LBUTTONDBLCLK)
        {
            ShowFromTray();
        }
    }

    void MainWindow::OnTrayMenuCmd(int cmd)
    {
        switch (cmd)
        {
        case 1001: // CMD_SHOW
            ShowFromTray();
            break;
        case 1002: // CMD_MARK_ALL_READ
            NotificationService::Instance().MarkAllRead();
            break;
        case 1003: // CMD_CLEAR_ALL
            NotificationService::Instance().ClearAll();
            break;
        case 1004: // CMD_SETTINGS
            ShowFromTray();
            // TODO: 打开设置页
            break;
        case 1005: // CMD_EXIT
            PostQuitMessage(0);
            break;
        }
    }

    void MainWindow::MinimizeBtn_Click(IInspectable const&, RoutedEventArgs const&)
    {
        auto& cfg = ConfigManager::Instance().Get();
        if (cfg.minimizeToTray)
        {
            HideToTray();
        }
        else
        {
            ShowWindow(m_hwnd, SW_MINIMIZE);
        }
    }

    void MainWindow::CloseBtn_Click(IInspectable const&, RoutedEventArgs const&)
    {
        auto& cfg = ConfigManager::Instance().Get();
        if (cfg.closeToTray)
        {
            HideToTray();
        }
        else
        {
            Close();
        }
    }

    void MainWindow::HideToTray()
    {
        ShowWindow(m_hwnd, SW_HIDE);
        m_trayIcon->ShowBalloon(L"WinNotify", L"应用已最小化到系统托盘", 2000);
    }

    void MainWindow::ShowFromTray()
    {
        ShowWindow(m_hwnd, SW_SHOW);
        SetForegroundWindow(m_hwnd);
    }
}

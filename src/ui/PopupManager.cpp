#include "pch.h"
#include "ui/PopupManager.h"
#include "services/ConfigManager.h"
#include "services/NotificationService.h"

using namespace winrt;
using namespace Microsoft::UI::Xaml;
using namespace Microsoft::UI::Xaml::Media::Animation;

// ===== PopupWindow =====

PopupWindow::PopupWindow()
{
    CreateWindow();
}

PopupWindow::~PopupWindow()
{
    m_visible = false;
    if (m_autoCloseThread.joinable()) m_autoCloseThread.detach();
    if (m_hwnd)
    {
        // 窗口会随 XAML 岛生命周期管理
    }
}

void PopupWindow::CreateWindow()
{
    m_window = Window();
    m_window.ExtendsContentIntoTitleBar(true);
    m_window.SystemBackdrop(Microsoft::UI::Xaml::Media::MicaBackdrop());

    // 设置无边框
    auto nativeWnd = m_window.AppWindow();
    auto presenter = nativeWnd.Presenter().as<Microsoft::UI::Windowing::AppWindowPresenter>();
    if (presenter)
    {
        auto config = presenter.GetConfiguration();
        config.HasBorder(false);
        config.HasTitleBar(false);
        config.IsAlwaysOnTop(true);
        presenter.ApplyConfiguration(config);
    }

    m_popup = WinNotify::NotificationPopup();
    m_window.Content(m_popup);

    // 获取窗口句柄
    auto windowNative = m_window.as<IWindowNative>();
    windowNative->get_WindowHandle(&m_hwnd);

    // 设置窗口样式 - 无边框、工具窗口（不显示在任务栏）
    DWORD style = GetWindowLong(m_hwnd, GWL_STYLE);
    style &= ~(WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU);
    SetWindowLong(m_hwnd, GWL_STYLE, style);

    DWORD exStyle = GetWindowLong(m_hwnd, GWL_EXSTYLE);
    exStyle |= WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE;
    SetWindowLong(m_hwnd, GWL_EXSTYLE, exStyle);

    // 透明背景
    m_window.Background(SolidColorBrush(Windows::UI::Colors::Transparent()));

    // 事件绑定
    m_popup.Closed([this](auto&&, auto&& id) {
        m_closed(*this, id);
    });
    m_popup.Clicked([this](auto&&, auto&& id) {
        m_clicked(*this, id);
    });
}

void PopupWindow::Show(const Notification& n, int x, int y, int animMs)
{
    m_popup.SetNotification(n);
    m_x = x;
    m_y = y;
    m_visible = true;

    // 初始位置（屏幕外）
    SetWindowPos(m_hwnd, HWND_TOPMOST, x + 100, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
    m_window.Activate();

    // 估算高度
    auto& cfg = ConfigManager::Instance().Get();
    m_height = 100;
    SetWindowPos(m_hwnd, HWND_TOPMOST, x, y, cfg.popup.width + 32, m_height + 28,
        SWP_NOACTIVATE);

    // 滑入动画
    SetupAnimation();
}

void PopupWindow::SetupAnimation()
{
    // 使用 Composition 动画做滑入效果
    try
    {
        auto compositor = Microsoft::UI::Xaml::Media::Animation::AnimatedVisualPlayer();
        // 简化：用 Win32 动画定时器实现滑入
        auto& cfg = ConfigManager::Instance().Get();
        int duration = cfg.popup.animationDurationMs;
        int startX = m_x + 80;
        int endX = m_x;
        int steps = 20;
        int stepMs = duration / steps;

        std::thread anim([this, startX, endX, steps, stepMs]() {
            for (int i = 0; i <= steps && m_visible; i++)
            {
                double t = (double)i / steps;
                // ease out
                double ease = 1 - pow(1 - t, 3);
                int curX = startX + (int)((endX - startX) * ease);
                SetWindowPos(m_hwnd, HWND_TOPMOST, curX, m_y, 0, 0,
                    SWP_NOSIZE | SWP_NOACTIVATE);
                std::this_thread::sleep_for(std::chrono::milliseconds(stepMs));
            }
        });
        anim.detach();
    }
    catch (...) {}
}

void PopupWindow::Hide(int animMs)
{
    m_visible = false;

    // 滑出动画
    int steps = 15;
    int stepMs = animMs / steps;
    int startX = m_x;
    int endX = m_x + 120;

    std::thread anim([this, startX, endX, steps, stepMs]() {
        for (int i = 0; i <= steps; i++)
        {
            double t = (double)i / steps;
            double ease = t * t; // ease in
            int curX = startX + (int)((endX - startX) * ease);
            SetWindowPos(m_hwnd, HWND_TOPMOST, curX, m_y, 0, 0,
                SWP_NOSIZE | SWP_NOACTIVATE);
            std::this_thread::sleep_for(std::chrono::milliseconds(stepMs));
        }
        m_window.Close();
    });
    anim.detach();
}

void PopupWindow::Move(int x, int y, int animMs)
{
    m_y = y;
    m_x = x;

    int steps = 15;
    int stepMs = animMs / steps;
    int startY = m_y; // 当前y
    // 简化：直接移动
    SetWindowPos(m_hwnd, HWND_TOPMOST, x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
}

void PopupWindow::SetAutoClose(int ms)
{
    if (m_autoCloseThread.joinable()) m_autoCloseThread.detach();

    m_autoCloseThread = std::jthread([this, ms]() {
        std::this_thread::sleep_for(std::chrono::milliseconds(ms));
        if (m_visible)
        {
            m_closed(*this, L"");
        }
    });
}

// ===== PopupManager =====

PopupManager& PopupManager::Instance()
{
    static PopupManager inst;
    return inst;
}

void PopupManager::Init()
{
    // 获取主显示器尺寸
    HMONITOR hmon = MonitorFromWindow(GetDesktopWindow(), MONITOR_DEFAULTTOPRIMARY);
    MONITORINFO mi = { sizeof(mi) };
    if (GetMonitorInfo(hmon, &mi))
    {
        m_screenWidth = mi.rcWork.right - mi.rcWork.left;
        m_screenHeight = mi.rcWork.bottom - mi.rcWork.top;
    }
}

void PopupManager::ShowNotification(const Notification& n)
{
    std::lock_guard lock(m_mtx);

    auto& cfg = ConfigManager::Instance().Get();

    // 超过最大数量则忽略
    if ((int)m_popups.size() >= cfg.popup.maxVisible) return;

    auto popup = std::make_unique<PopupWindow>();
    auto* raw = popup.get();

    raw->Closed([this](auto&&, auto&& id) {
        OnPopupClosed(id);
    });
    raw->Clicked([this](auto&&, auto&& id) {
        OnPopupClicked(id);
    });

    int x = m_screenWidth - cfg.popup.width - cfg.popup.marginRight - 32;
    int y = m_screenHeight - cfg.popup.marginBottom - raw->Height() - 28;

    // 调整位置 - 堆叠在已有弹窗上方
    for (auto& p : m_popups)
    {
        if (p->IsVisible())
            y -= p->Height() + cfg.popup.spacing;
    }

    raw->Show(n, x, y, cfg.popup.animationDurationMs);
    raw->SetAutoClose(cfg.popup.displayDurationMs);

    m_popups.push_back(std::move(popup));
}

void PopupManager::CloseAll()
{
    std::lock_guard lock(m_mtx);
    auto& cfg = ConfigManager::Instance().Get();
    for (auto& p : m_popups)
    {
        if (p->IsVisible())
            p->Hide(cfg.popup.animationDurationMs);
    }
}

void PopupManager::Rearrange()
{
    auto& cfg = ConfigManager::Instance().Get();
    int y = m_screenHeight - cfg.popup.marginBottom - 128;
    int x = m_screenWidth - cfg.popup.width - cfg.popup.marginRight - 32;

    for (auto& p : m_popups)
    {
        if (p->IsVisible())
        {
            p->Move(x, y, cfg.popup.animationDurationMs);
            y -= p->Height() + cfg.popup.spacing;
        }
    }
}

void PopupManager::OnPopupClosed(const hstring& id)
{
    std::lock_guard lock(m_mtx);

    std::erase_if(m_popups, [&](auto& p) {
        return !p->IsVisible() || p.get() == nullptr;
    });

    Rearrange();
}

void PopupManager::OnPopupClicked(const hstring& id)
{
    auto& svc = NotificationService::Instance();
    svc.MarkRead(id.c_str());

    // 打开链接
    Notification n;
    bool found = false;
    for (auto& notif : svc.GetAll())
    {
        if (notif.id == id.c_str())
        {
            n = notif;
            found = true;
            break;
        }
    }

    if (found && !n.actionUrl.empty())
    {
        Windows::System::Launcher::LaunchUriAsync(Windows::Foundation::Uri(n.actionUrl));
    }

    // 关闭弹窗
    OnPopupClosed(id);
}

#include "pch.h"
#include "MainWindow.xaml.h"
#include "Services/NotificationService.h"
#include "Models/AppConfig.h"

using namespace winrt;
using namespace Microsoft::UI::Xaml;
using namespace Microsoft::UI::Xaml::Controls;
using namespace Microsoft::UI::Xaml::Media;
using namespace Microsoft::UI::Xaml::Media::Animation;
using namespace Microsoft::UI::Xaml::Input;
using namespace winrt::Windows::UI;

namespace winrt::NotificationManager::implementation {

MainWindow* MainWindow::s_current = nullptr;

MainWindow::MainWindow() {
    InitializeComponent();
    s_current = this;
    _cfg = nm::AppConfig::Load("config.json");

    InitWindow();
    InitService();
    InitTray();
}

void MainWindow::InitWindow() {
    auto wnd = this->as<IWindowNative>();
    if (wnd) {
        wnd->get_WindowHandle(&_hwnd);
        SetWindowSubclass(_hwnd, [](HWND hwnd, UINT msg, WPARAM wp, LPARAM lp,
                                     UINT_PTR, DWORD_PTR ref) -> LRESULT {
            auto* self = reinterpret_cast<MainWindow*>(ref);
            if (msg == nm::TrayIcon::WM_TASKBAR) {
                auto lpm = LOWORD(lp);
                if (lpm == WM_RBUTTONUP) {
                    // Right-click: show context menu via tray callback suppressed, use popup menu
                    HMENU menu = CreatePopupMenu();
                    AppendMenuW(menu, MF_STRING, nm::TrayIcon::CMD_SHOW, L"Show");
                    AppendMenuW(menu, MF_STRING, nm::TrayIcon::CMD_CLEAR_ALL, L"Clear All");
                    AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
                    AppendMenuW(menu, MF_STRING, nm::TrayIcon::CMD_EXIT, L"Exit");
                    POINT pt; GetCursorPos(&pt);
                    SetForegroundWindow(hwnd);
                    auto cmd = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_NONOTIFY,
                                              pt.x, pt.y, 0, hwnd, nullptr);
                    DestroyMenu(menu);
                    if (cmd == nm::TrayIcon::CMD_SHOW) {
                        self->DispatcherQueue().TryEnqueue([self] {
                            self->Activate();
                        });
                    } else if (cmd == nm::TrayIcon::CMD_CLEAR_ALL) {
                        self->DispatcherQueue().TryEnqueue([self] {
                            self->OnClearAll(nullptr, nullptr);
                        });
                    } else if (cmd == nm::TrayIcon::CMD_EXIT) {
                        self->DispatcherQueue().TryEnqueue([self] {
                            self->Close();
                        });
                    }
                } else if (lpm == WM_LBUTTONDBLCLK) {
                    self->DispatcherQueue().TryEnqueue([self] {
                        self->Activate();
                    });
                }
                return 0;
            }
            if (msg == WM_CLOSE && self->_cfg.closeToTray) {
                // Minimize to tray instead of closing
                self->DispatcherQueue().TryEnqueue([self] {
                    ShowWindow(self->_hwnd, SW_HIDE);
                });
                return 0;
            }
            return DefSubclassProc(hwnd, msg, wp, lp);
        }, 1, reinterpret_cast<DWORD_PTR>(this));
    }

    // Apply rounded corners
    auto appWnd = GetAppWindowForCurrentWindow();
    if (appWnd) {
        appWnd.TitleBar().ExtendsContentIntoTitleBar(true);
        appWnd.TitleBar().ButtonBackgroundColor(Colors::Transparent());
        appWnd.TitleBar().ButtonInactiveBackgroundColor(Colors::Transparent());
    }
}

void MainWindow::InitTray() {
    _tray = std::make_unique<nm::TrayIcon>(_hwnd);
    _tray->Show();
}

void MainWindow::InitService() {
    auto& svc = nm::NotificationService::Instance();

    svc.OnNewNotification([this](const nm::NotificationItem& item) {
        DispatcherQueue().TryEnqueue([this, item] {
            ShowToast(item);
            RefreshList();
            UpdateBadge();
        });
    });

    svc.OnConnectionState([this](bool connected) {
        DispatcherQueue().TryEnqueue([this, connected] {
            UpdateConnStatus(connected);
        });
    });

    if (!_cfg.wsUrl.empty()) {
        svc.Start(_cfg.wsUrl);
    }

    // Periodic refresh
    _refreshTimer = DispatcherTimer();
    _refreshTimer.Interval(std::chrono::seconds(30));
    _refreshTimer.Tick([this](auto&, auto&) {
        RefreshList();
    });
    _refreshTimer.Start();

    RefreshList();
    UpdateBadge();
}

void MainWindow::RefreshList() {
    auto& svc = nm::NotificationService::Instance();
    _history = svc.GetHistory();

    // Apply filter
    if (_filterActive) {
        _history.erase(std::remove_if(_history.begin(), _history.end(),
            [this](auto& n) { return n.category != _filter; }), _history.end());
    }

    // Apply search
    if (!_searchText.empty()) {
        auto lower = _searchText;
        std::transform(lower.begin(), lower.end(), lower.begin(), ::tolower);
        _history.erase(std::remove_if(_history.begin(), _history.end(),
            [&lower](auto& n) {
                auto t = n.title; auto b = n.body;
                std::transform(t.begin(), t.end(), t.begin(), ::tolower);
                std::transform(b.begin(), b.end(), b.begin(), ::tolower);
                return t.find(lower) == std::string::npos &&
                       b.find(lower) == std::string::npos;
            }), _history.end());
    }

    NotifList().Children().Clear();
    EmptyLabel().Visibility(_history.empty() ? Visibility::Visible : Visibility::Collapsed);

    if (_history.empty()) return;

    // Group by date
    std::string lastGroup;
    for (auto& item : _history) {
        auto group = FormatDateGroup(item.timestamp);
        if (group != lastGroup) {
            lastGroup = group;
            TextBlock header;
            header.Text(winrt::to_hstring(group));
            header.Style(Resources().Lookup(box_value(L"DateHeaderStyle")).as<Style>());
            NotifList().Children().Append(header);
        }
        NotifList().Children().Append(BuildNotifCard(item));
    }
}

void MainWindow::ShowToast(const nm::NotificationItem& item) {
    auto toast = BuildToast(item);
    ToastPanel().Children().InsertAt(0, toast);

    // Slide-in animation
    auto slideAnim = Storyboard();
    auto translate = toast.Translation();
    auto anim = Composition::CompositionAnimation();
    // Using built-in entrance animation theme transition instead
    auto transitions = TransitionCollection();
    auto edgeTransition = EdgeUIThemeTransition();
    edgeTransition.Edge(ElementTheme::Right);
    transitions.Append(edgeTransition);
    toast.Transitions(transitions);

    // Auto-dismiss timer
    DispatcherTimer timer;
    timer.Interval(std::chrono::milliseconds(_cfg.popupDuration));
    auto id = item.id;
    timer.Tick([this, id](auto&, auto&) {
        DismissToast(id);
    });
    timer.Start();

    ToastItem ti;
    ti.id = item.id;
    ti.element = toast;
    ti.timer = timer;
    _toasts.push_back(ti);

    // Limit toast count
    while (_toasts.size() > static_cast<size_t>(_cfg.popupMaxCount)) {
        DismissToast(_toasts.front().id);
    }

    ToastOverlay().IsOpen(true);
}

void MainWindow::DismissToast(const std::string& id) {
    auto it = std::find_if(_toasts.begin(), _toasts.end(),
        [&](auto& t) { return t.id == id; });
    if (it != _toasts.end()) {
        if (it->timer) it->timer.Stop();
        auto idx = std::distance(_toasts.begin(), it);
        if (idx < static_cast<int>(ToastPanel().Children().Size())) {
            ToastPanel().Children().RemoveAt(static_cast<uint32_t>(idx));
        }
        _toasts.erase(it);
    }
    if (_toasts.empty()) {
        ToastOverlay().IsOpen(false);
    }
}

FrameworkElement MainWindow::BuildToast(const nm::NotificationItem& item) {
    Border border;
    border.Background(SolidColorBrush(ColorHelper::FromArgb(0xF2, 0xFF, 0xFF, 0xFF)));
    border.CornerRadius(CornerRadius(8));
    border.BorderBrush(SolidColorBrush(ColorHelper::FromArgb(0x40, 0, 0, 0)));
    border.BorderThickness(ThicknessHelper::FromUniformLength(1));
    border.Width(340);
    border.Shadow(ThemeShadow());

    StackPanel panel;
    panel.Padding(ThicknessHelper::FromLengths(12, 10, 12, 10));

    // Header row: category dot + title + close
    StackPanel headerRow;
    headerRow.Orientation(Orientation::Horizontal);

    // Category indicator
    Ellipse dot;
    dot.Width(8); dot.Height(8);
    auto catColor = GetCatColor(item.category);
    dot.Fill(SolidColorBrush(ColorHelper::FromArgb(0xFF,
        static_cast<uint8_t>(std::stoi(catColor.substr(0,2), nullptr, 16)),
        static_cast<uint8_t>(std::stoi(catColor.substr(2,2), nullptr, 16)),
        static_cast<uint8_t>(std::stoi(catColor.substr(4,2), nullptr, 16)))));
    dot.VerticalAlignment(VerticalAlignment::Center);
    dot.Margin(ThicknessHelper::FromLengths(0, 0, 8, 0));

    TextBlock title;
    title.Text(winrt::to_hstring(item.title));
    title.FontWeight(Text::FontWeights::SemiBold());
    title.FontSize(13);
    title.Foreground(SolidColorBrush(ColorHelper::FromArgb(0xFF, 0x1A, 0x1A, 0x1A)));
    title.VerticalAlignment(VerticalAlignment::Center);

    Button closeBtn;
    closeBtn.Content(box_value(L"\uE711"));
    closeBtn.FontFamily(Media::FontFamily(L"Segoe MDL2 Assets"));
    closeBtn.FontSize(10);
    closeBtn.Width(24); closeBtn.Height(24);
    closeBtn.Background(SolidColorBrush(Colors::Transparent()));
    closeBtn.HorizontalAlignment(HorizontalAlignment::Right);
    auto closeId = item.id;
    closeBtn.Click([this, closeId](auto&, auto&) { DismissToast(closeId); });

    headerRow.Children().Append(dot);
    headerRow.Children().Append(title);
    headerRow.Children().Append(closeBtn);

    // Body
    TextBlock body;
    body.Text(winrt::to_hstring(item.body));
    body.FontSize(12);
    body.Foreground(SolidColorBrush(ColorHelper::FromArgb(0xFF, 0x66, 0x66, 0x66)));
    body.TextWrapping(TextWrapping::Wrap);
    body.MaxLines(2);
    body.TextTrimming(TextTrimming::CharacterEllipsis);
    body.Margin(ThicknessHelper::FromLengths(0, 4, 0, 0));

    // Action button (if has action)
    if (!item.actionUrl.empty()) {
        HyperlinkButton action;
        auto label = item.actionLabel.empty() ? "Open" : item.actionLabel;
        action.Content(box_value(winrt::to_hstring(label)));
        action.FontSize(11);
        action.Margin(ThicknessHelper::FromLengths(0, 6, 0, 0));
        auto actionUrl = item.actionUrl;
        action.Click([this, actionUrl](auto&, auto&) {
            Windows::System::Launcher::LaunchUriAsync(
                Windows::Foundation::Uri(winrt::to_hstring(actionUrl)));
        });
        panel.Children().Append(action);
    }

    panel.Children().Append(headerRow);
    panel.Children().Append(body);

    // Click on toast to open notification center
    auto notifId = item.id;
    border.Tapped([this, notifId](auto&, auto&) {
        nm::NotificationService::Instance().MarkRead(notifId);
        DismissToast(notifId);
        Activate();
        RefreshList();
    });

    border.Child(panel);
    return border;
}

FrameworkElement MainWindow::BuildNotifCard(const nm::NotificationItem& item) {
    Border card;
    card.Background(SolidColorBrush(ColorHelper::FromArgb(0xFF, 0xFF, 0xFF, 0xFF)));
    card.CornerRadius(CornerRadius(8));
    card.Margin(ThicknessHelper::FromLengths(0, 0, 0, 6));
    card.BorderBrush(SolidColorBrush(ColorHelper::FromArgb(0x20, 0, 0, 0)));
    card.BorderThickness(ThicknessHelper::FromUniformLength(1));

    Grid grid;
    grid.ColumnDefinitions().Append(ColumnDefinition{4, GridUnitType::Pixel}); // dot
    grid.ColumnDefinitions().Append(ColumnDefinition{1, GridUnitType::Star});   // content
    grid.ColumnDefinitions().Append(ColumnDefinition{40, GridUnitType::Pixel}); // delete
    grid.Padding(ThicknessHelper::FromLengths(8, 10, 4, 10));

    // Unread dot
    if (!item.read) {
        Ellipse dot;
        dot.Width(8); dot.Height(8);
        dot.Fill(SolidColorBrush(ColorHelper::FromArgb(0xFF, 0x00, 0x78, 0xD4)));
        dot.VerticalAlignment(VerticalAlignment::Top);
        dot.Margin(ThicknessHelper::FromLengths(0, 4, 8, 0));
        Grid::SetColumn(dot, 0);
        grid.Children().Append(dot);
    }

    // Content
    StackPanel content;
    Grid::SetColumn(content, 1);
    TextBlock title;
    title.Text(winrt::to_hstring(item.title));
    title.FontWeight(item.read ? Text::FontWeights::Normal() : Text::FontWeights::SemiBold());
    title.FontSize(14);
    title.Foreground(SolidColorBrush(ColorHelper::FromArgb(0xFF, 0x1A, 0x1A, 0x1A)));

    TextBlock body;
    body.Text(winrt::to_hstring(item.body));
    body.FontSize(12);
    body.Foreground(SolidColorBrush(ColorHelper::FromArgb(0xFF, 0x66, 0x66, 0x66)));
    body.TextTrimming(TextTrimming::CharacterEllipsis);
    body.MaxLines(1);

    TextBlock time;
    time.Text(winrt::to_hstring(FormatTime(item.timestamp)));
    time.FontSize(11);
    time.Foreground(SolidColorBrush(ColorHelper::FromArgb(0xFF, 0x99, 0x99, 0x99)));
    time.Margin(ThicknessHelper::FromLengths(0, 2, 0, 0));

    content.Children().Append(title);
    content.Children().Append(body);
    content.Children().Append(time);

    // Delete button
    Button delBtn;
    delBtn.Content(box_value(L"\uE74D"));
    delBtn.FontFamily(Media::FontFamily(L"Segoe MDL2 Assets"));
    delBtn.FontSize(12);
    delBtn.Width(28); delBtn.Height(28);
    delBtn.Background(SolidColorBrush(Colors::Transparent()));
    delBtn.Foreground(SolidColorBrush(ColorHelper::FromArgb(0x80, 0x99, 0x99, 0x99)));
    delBtn.VerticalAlignment(VerticalAlignment::Center);
    delBtn.HorizontalAlignment(HorizontalAlignment::Right);
    Grid::SetColumn(delBtn, 2);
    auto delId = item.id;
    delBtn.Click([this, delId](auto&, auto&) { OnDeleteNotif(delId); });

    grid.Children().Append(content);
    grid.Children().Append(delBtn);
    card.Child(grid);

    // Click handler
    auto tapItem = item;
    card.Tapped([this, tapItem](auto&, auto&) { OnNotifTap(tapItem); });

    return card;
}

void MainWindow::OnNotifTap(const nm::NotificationItem& item) {
    nm::NotificationService::Instance().MarkRead(item.id);

    if (!item.actionUrl.empty()) {
        Windows::System::Launcher::LaunchUriAsync(
            Windows::Foundation::Uri(winrt::to_hstring(item.actionUrl)));
    }

    RefreshList();
    UpdateBadge();
}

void MainWindow::OnDeleteNotif(const std::string& id) {
    nm::NotificationService::Instance().DeleteItem(id);
    RefreshList();
    UpdateBadge();
}

// === Event Handlers ===

void MainWindow::OnSearchChanged(IInspectable const&, TextBoxTextChangedEventArgs const&) {
    _searchText = winrt::to_string(SearchBox().Text());
    RefreshList();
}

void MainWindow::OnFilterAll(IInspectable const&, RoutedEventArgs const&) {
    _filterActive = false;
    UpdateFilterUI();
    RefreshList();
}

void MainWindow::OnFilterMsg(IInspectable const&, RoutedEventArgs const&) {
    _filter = nm::NotifCategory::Message;
    _filterActive = true;
    UpdateFilterUI();
    RefreshList();
}

void MainWindow::OnFilterRem(IInspectable const&, RoutedEventArgs const&) {
    _filter = nm::NotifCategory::Reminder;
    _filterActive = true;
    UpdateFilterUI();
    RefreshList();
}

void MainWindow::OnFilterSys(IInspectable const&, RoutedEventArgs const&) {
    _filter = nm::NotifCategory::System;
    _filterActive = true;
    UpdateFilterUI();
    RefreshList();
}

void MainWindow::UpdateFilterUI() {
    auto activeBrush = SolidColorBrush(ColorHelper::FromArgb(0xFF, 0x00, 0x78, 0xD4));
    auto activeFg = SolidColorBrush(Colors::White());
    auto inactiveBg = SolidColorBrush(ColorHelper::FromArgb(0x10, 0, 0, 0));
    auto inactiveFg = SolidColorBrush(ColorHelper::FromArgb(0xFF, 0x66, 0x66, 0x66));

    auto setBtn = [&](Button& btn, bool active) {
        btn.Background(active ? activeBrush : inactiveBg);
        btn.Foreground(active ? activeFg : inactiveFg);
    };

    bool all = !_filterActive;
    setBtn(TabAll(), all);
    setBtn(TabMsg(), _filterActive && _filter == nm::NotifCategory::Message);
    setBtn(TabRem(), _filterActive && _filter == nm::NotifCategory::Reminder);
    setBtn(TabSys(), _filterActive && _filter == nm::NotifCategory::System);
}

void MainWindow::OnClearAll(IInspectable const&, RoutedEventArgs const&) {
    nm::NotificationService::Instance().ClearAll();
    RefreshList();
    UpdateBadge();
}

void MainWindow::OnSettings(IInspectable const&, RoutedEventArgs const&) {
    // Open config file location
    namespace fs = std::filesystem;
    auto cfgPath = fs::absolute("config.json");
    if (fs::exists(cfgPath)) {
        ShellExecuteW(nullptr, L"open", cfgPath.c_str(), nullptr, nullptr, SW_SHOW);
    } else {
        // Create default config
        _cfg.Save("config.json");
        cfgPath = fs::absolute("config.json");
        ShellExecuteW(nullptr, L"open", cfgPath.c_str(), nullptr, nullptr, SW_SHOW);
    }
}

void MainWindow::UpdateBadge() {
    auto count = nm::NotificationService::Instance().UnreadCount();
    UnreadBadge().Text(count > 0
        ? winrt::to_hstring(std::to_string(count) + " unread")
        : L"No unread notifications");
    if (_tray) _tray->UpdateUnreadBadge(static_cast<int>(count));
}

void MainWindow::UpdateConnStatus(bool connected) {
    ConnDot().Fill(SolidColorBrush(ColorHelper::FromArgb(0xFF,
        connected ? 0x10 : 0xCC,
        connected ? 0x7C : 0xCC,
        connected ? 0x00 : 0xCC)));
    ConnStatus().Text(connected ? L"Connected" : L"Disconnected");
}

// === Formatting Helpers ===

std::string MainWindow::FormatTime(uint64_t ts) {
    auto now = std::chrono::system_clock::now();
    auto t = std::chrono::system_clock::time_point(std::chrono::milliseconds(ts));
    auto diff = std::chrono::duration_cast<std::chrono::minutes>(now - t).count();

    if (diff < 1) return "Just now";
    if (diff < 60) return std::to_string(diff) + "m ago";

    auto hours = diff / 60;
    if (hours < 24) return std::to_string(hours) + "h ago";

    auto days = hours / 24;
    if (days < 7) return std::to_string(days) + "d ago";

    auto time_t_val = std::chrono::system_clock::to_time_t(t);
    std::tm tm_val;
    localtime_s(&tm_val, &time_t_val);
    std::ostringstream oss;
    oss << std::put_time(&tm_val, "%m/%d");
    return oss.str();
}

std::string MainWindow::FormatDateGroup(uint64_t ts) {
    auto now = std::chrono::system_clock::now();
    auto today = std::chrono::floor<std::chrono::days>(now);
    auto notifDay = std::chrono::floor<std::chrono::days>(
        std::chrono::system_clock::time_point(std::chrono::milliseconds(ts)));

    auto diff = (today - notifDay).count();
    if (diff == 0) return "Today";
    if (diff == 1) return "Yesterday";
    if (diff < 7) return "This Week";
    if (diff < 30) return "This Month";
    return "Older";
}

std::string MainWindow::GetCatColor(nm::NotifCategory cat) {
    switch (cat) {
    case nm::NotifCategory::Message: return "0078D4";
    case nm::NotifCategory::Reminder: return "FF8C00";
    case nm::NotifCategory::System: return "E81123";
    default: return "886CE4";
    }
}

std::string MainWindow::GetCatIcon(nm::NotifCategory cat) {
    switch (cat) {
    case nm::NotifCategory::Message: return "\uE8BD";
    case nm::NotifCategory::Reminder: return "\uE823";
    case nm::NotifCategory::System: return "\uE7BA";
    default: return "\uE8A5";
    }
}

} // namespace winrt::NotificationManager::implementation

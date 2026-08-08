#pragma once
#include "MainWindow.xaml.g.h"
#include "Models/NotificationItem.h"
#include "Models/AppConfig.h"
#include "Helpers/TrayIcon.h"

namespace winrt::NotificationManager::implementation {

struct ToastItem {
    std::string id;
    winrt::Windows::UI::Xaml::FrameworkElement element{ nullptr };
    winrt::Windows::UI::Xaml::DispatcherTimer timer{ nullptr };
};

struct MainWindow : MainWindowT<MainWindow> {
    MainWindow();

    void OnSearchChanged(winrt::Windows::Foundation::IInspectable const&,
                         winrt::Microsoft::UI::Xaml::Controls::TextBoxTextChangedEventArgs const&);
    void OnFilterAll(winrt::Windows::Foundation::IInspectable const&,
                     winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnFilterMsg(winrt::Windows::Foundation::IInspectable const&,
                     winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnFilterRem(winrt::Windows::Foundation::IInspectable const&,
                     winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnFilterSys(winrt::Windows::Foundation::IInspectable const&,
                     winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnClearAll(winrt::Windows::Foundation::IInspectable const&,
                    winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnSettings(winrt::Windows::Foundation::IInspectable const&,
                    winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);

private:
    void InitTray();
    void InitService();
    void InitWindow();
    void RefreshList();
    void UpdateFilterUI();
    void UpdateBadge();
    void UpdateConnStatus(bool connected);
    void ShowToast(const nm::NotificationItem& item);
    void DismissToast(const std::string& id);
    winrt::Windows::UI::Xaml::FrameworkElement BuildToast(const nm::NotificationItem& item);
    winrt::Windows::UI::Xaml::FrameworkElement BuildNotifCard(const nm::NotificationItem& item);
    void OnNotifTap(const nm::NotificationItem& item);
    void OnDeleteNotif(const std::string& id);

    static std::string FormatTime(uint64_t ts);
    static std::string FormatDateGroup(uint64_t ts);
    std::string GetCatColor(nm::NotifCategory cat);
    std::string GetCatIcon(nm::NotifCategory cat);

    // Window message handler for tray icon
    static LRESULT CALLBACK WndProc(HWND, UINT, WPARAM, LPARAM);
    static MainWindow* s_current;

    HWND _hwnd = nullptr;
    std::unique_ptr<nm::TrayIcon> _tray;
    nm::NotifCategory _filter = nm::NotifCategory::Message;
    bool _filterActive = false;
    std::string _searchText;
    std::vector<nm::NotificationItem> _history;
    std::vector<ToastItem> _toasts;
    nm::AppConfig _cfg;
    winrt::Windows::UI::Xaml::DispatcherTimer _refreshTimer{ nullptr };
};

} // namespace winrt::NotificationManager::implementation

namespace winrt::NotificationManager::factory_implementation {
    struct MainWindow : MainWindowT<MainWindow, implementation::MainWindow> {};
}

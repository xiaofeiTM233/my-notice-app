#pragma once
#include "MainWindow.xaml.g.h"
#include "core/NotifyItem.h"

namespace winrt::WinNotify::implementation
{
    struct MainWindow : MainWindowT<MainWindow>
    {
        MainWindow();

        void ShowWindow();
        void HideWindow();
        void Refresh();
        void OnNotif(const winn::NotifyItem& it);

    private:
        void Rebuild();
        void RefreshBadge();
        void ShowItemMenu(const winn::NotifyItem& it, winrt::Microsoft::UI::Xaml::FrameworkElement anchor);
        void ActivateItem(const winn::NotifyItem& it);

        std::wstring m_query;
        int m_cat{ 0 };
    };
}
namespace winrt::WinNotify::factory_implementation
{
    struct MainWindow : MainWindowT<MainWindow, implementation::MainWindow>
    {
    };
}

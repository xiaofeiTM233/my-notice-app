#pragma once
#include "pch.h"
#include "NotifyItem.h"
#include "Config.h"

namespace winn
{
    class ToastMgr;

    class Toast
    {
    public:
        Toast(const Config& cfg, const NotifyItem& it, std::function<void(const NotifyItem&)> onAct);
        ~Toast();
        void Show();
        void Close();
        bool Visible() const { return m_visible; }

        std::function<void(Toast*)> onClosed;

    private:
        friend class ToastMgr;
        void SetIndex(int i) { m_index = i; }
        void Layout(int index);
        void BuildUi();
        void AddShadow();
        void SlideIn();

        const Config& m_cfg;
        NotifyItem m_item;
        std::function<void(const NotifyItem&)> m_onAct;
        winrt::Microsoft::UI::Xaml::Window m_win{ nullptr };
        winrt::Microsoft::UI::Xaml::DispatcherTimer m_timer{ nullptr };
        bool m_visible{ false };
        int m_index{ 0 };
    };

    class ToastMgr
    {
    public:
        explicit ToastMgr(const Config& cfg);
        void Push(const NotifyItem& it, std::function<void(const NotifyItem&)> onAct);
        void CloseAll();
        int Count() const { return (int)m_toasts.size(); }

    private:
        void Remove(Toast* t);
        void Relayout();

        const Config& m_cfg;
        std::vector<std::shared_ptr<Toast>> m_toasts;
    };
}

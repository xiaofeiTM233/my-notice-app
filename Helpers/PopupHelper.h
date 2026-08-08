#pragma once
#include "pch.h"
#include "../Models/NotificationItem.h"

namespace nm {

class PopupHelper {
public:
    struct PopupWindow {
        winrt::Microsoft::UI::Xaml::Window window{ nullptr };
        uint64_t createdAt = 0;
        std::string notifId;
    };

    static PopupHelper& Instance();

    void Show(const NotificationItem& item,
              winrt::Microsoft::UI::WindowId parentId,
              std::function<void(const std::string&)> onAction);

    void Dismiss(const std::string& id);
    void DismissAll();
    void SetDuration(int ms) { _duration = ms; }
    void SetMaxCount(int n) { _maxCount = n; }

private:
    PopupHelper() = default;
    void PositionWindow(winrt::Microsoft::UI::Xaml::Window& wnd, int index);
    void Cleanup();

    std::vector<PopupWindow> _popups;
    std::mutex _mtx;
    int _duration = 5000;
    int _maxCount = 3;
    int _spacing = 8;
};

} // namespace nm

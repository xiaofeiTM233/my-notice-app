#include "pch.h"
#include "PopupHelper.h"

namespace nm {

PopupHelper& PopupHelper::Instance() {
    static PopupHelper inst;
    return inst;
}

void PopupHelper::Show(const NotificationItem& item,
                        winrt::Microsoft::UI::WindowId parentId,
                        std::function<void(const std::string&)> onAction) {
    Cleanup();

    std::lock_guard lock(_mtx);
    if (_popups.size() >= static_cast<size_t>(_maxCount)) {
        // Dismiss oldest
        auto& oldest = _popups.front();
        if (oldest.window) {
            try { oldest.window.Close(); } catch (...) {}
        }
        _popups.erase(_popups.begin());
    }

    // Create popup window (using a ContentDialog-style approach in the main window instead)
    // For now, we delegate to the main window to show a popup overlay
    // The actual XAML popup is managed by MainWindow
    PopupWindow pw;
    pw.notifId = item.id;
    pw.createdAt = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::system_clock::now().time_since_epoch()).count();
    _popups.push_back(pw);
}

void PopupHelper::Dismiss(const std::string& id) {
    std::lock_guard lock(_mtx);
    _popups.erase(std::remove_if(_popups.begin(), _popups.end(),
        [&](auto& p) { return p.notifId == id; }), _popups.end());
}

void PopupHelper::DismissAll() {
    std::lock_guard lock(_mtx);
    for (auto& pw : _popups) {
        if (pw.window) {
            try { pw.window.Close(); } catch (...) {}
        }
    }
    _popups.clear();
}

void PopupHelper::PositionWindow(winrt::Microsoft::UI::Xaml::Window& wnd, int index) {
    // Position from bottom-right, stacking upward
    auto display = winrt::Windows::Graphics::Display::DisplayInformation::GetForCurrentView();
    // Since this is a secondary window, we need to get screen bounds differently
    // This positioning is handled in the main window's popup overlay instead
}

void PopupHelper::Cleanup() {
    auto now = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::system_clock::now().time_since_epoch()).count();
    _popups.erase(std::remove_if(_popups.begin(), _popups.end(),
        [&](auto& p) {
            return (now - p.createdAt) > static_cast<uint64_t>(_duration + 1000);
        }), _popups.end());
}

} // namespace nm

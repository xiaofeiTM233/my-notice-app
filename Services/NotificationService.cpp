#include "pch.h"
#include "NotificationService.h"
#include "WebSocketClient.h"

namespace nm {

NotificationService& NotificationService::Instance() {
    static NotificationService inst;
    return inst;
}

void NotificationService::Start(const std::string& wsUrl) {
    _ws = std::make_unique<WebSocketClient>();

    _ws->OnMessage([this](const std::string& msg) {
        ProcessMessage(msg);
    });

    _ws->OnStateChange([this](bool connected) {
        if (_onConnState) _onConnState(connected);
    });

    _ws->Connect(wsUrl);
}

void NotificationService::Stop() {
    if (_ws) {
        _ws->Disconnect();
        _ws.reset();
    }
}

void NotificationService::ProcessMessage(const std::string& msg) {
    try {
        auto json = winrt::Windows::Data::Json::JsonObject::Parse(winrt::to_hstring(msg));
        auto item = NotificationItem::FromJson(json);

        if (item.id.empty()) return;
        if (item.timestamp == 0) {
            item.timestamp = std::chrono::duration_cast<std::chrono::milliseconds>(
                std::chrono::system_clock::now().time_since_epoch()).count();
        }

        AddToHistory(item);
        if (_onNew) _onNew(item);
    }
    catch (...) {}
}

void NotificationService::AddToHistory(const NotificationItem& item) {
    std::lock_guard lock(_mtx);
    auto it = std::find_if(_history.begin(), _history.end(),
        [&](auto& n) { return n.id == item.id; });
    if (it != _history.end()) {
        *it = item;
    } else {
        _history.insert(_history.begin(), item);
        while (_history.size() > static_cast<size_t>(_maxHistory)) {
            _history.pop_back();
        }
    }
}

std::vector<NotificationItem> NotificationService::GetHistory() const {
    std::lock_guard lock(_mtx);
    return _history;
}

std::vector<NotificationItem> NotificationService::GetUnread() const {
    std::lock_guard lock(_mtx);
    std::vector<NotificationItem> unread;
    std::copy_if(_history.begin(), _history.end(), std::back_inserter(unread),
        [](auto& n) { return !n.read; });
    return unread;
}

void NotificationService::MarkRead(const std::string& id) {
    std::lock_guard lock(_mtx);
    auto it = std::find_if(_history.begin(), _history.end(),
        [&](auto& n) { return n.id == id; });
    if (it != _history.end()) it->read = true;
}

void NotificationService::MarkAllRead() {
    std::lock_guard lock(_mtx);
    for (auto& n : _history) n.read = true;
}

void NotificationService::DeleteItem(const std::string& id) {
    std::lock_guard lock(_mtx);
    _history.erase(std::remove_if(_history.begin(), _history.end(),
        [&](auto& n) { return n.id == id; }), _history.end());
}

void NotificationService::ClearAll() {
    std::lock_guard lock(_mtx);
    _history.clear();
}

size_t NotificationService::UnreadCount() const {
    std::lock_guard lock(_mtx);
    return std::count_if(_history.begin(), _history.end(),
        [](auto& n) { return !n.read; });
}

bool NotificationService::IsConnected() const {
    return _ws && _ws->IsConnected();
}

} // namespace nm

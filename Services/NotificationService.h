#pragma once
#include "pch.h"
#include "../Models/NotificationItem.h"

namespace nm {

using NotifHandler = std::function<void(const NotificationItem&)>;

class NotificationService {
public:
    static NotificationService& Instance();

    void Start(const std::string& wsUrl);
    void Stop();

    void AddToHistory(const NotificationItem& item);
    std::vector<NotificationItem> GetHistory() const;
    std::vector<NotificationItem> GetUnread() const;

    void MarkRead(const std::string& id);
    void MarkAllRead();
    void DeleteItem(const std::string& id);
    void ClearAll();

    size_t UnreadCount() const;
    void OnNewNotification(NotifHandler h) { _onNew = std::move(h); }
    void OnConnectionState(StateHandler h) { _onConnState = std::move(h); }

    bool IsConnected() const;

private:
    NotificationService() = default;
    void ProcessMessage(const std::string& msg);

    std::vector<NotificationItem> _history;
    mutable std::mutex _mtx;
    NotifHandler _onNew;
    StateHandler _onConnState;
    std::unique_ptr<class WebSocketClient> _ws;
    int _maxHistory = 500;
};

} // namespace nm

#pragma once
#include "pch.h"
#include "models/Notification.h"
#include "services/BackendClient.h"

class NotificationService
{
public:
    static NotificationService& Instance();

    void Init();
    void Shutdown();

    // 通知管理
    void AddNotification(const Notification& n);
    void RemoveNotification(const std::wstring& id);
    void ClearAll();
    void MarkRead(const std::wstring& id);
    void MarkAllRead();

    // 查询
    const std::vector<Notification>& GetAll() const { return m_notifications; }
    std::vector<Notification> Search(const std::wstring& keyword,
        NotifyCategory cat = static_cast<NotifyCategory>(-1),
        bool onlyUnread = false) const;
    int UnreadCount() const;
    std::vector<std::wstring> GetGroups() const;

    // 事件回调
    using NotifyEvent = std::function<void(const Notification&)>;
    using ListEvent = std::function<void()>;
    void SetOnPopup(NotifyEvent cb) { m_onPopup = std::move(cb); }
    void SetOnListChanged(ListEvent cb) { m_onListChanged = std::move(cb); }
    void SetOnUnreadChanged(ListEvent cb) { m_onUnreadChanged = std::move(cb); }

    // 过滤
    bool ShouldShow(const Notification& n) const;

    BackendClient& Backend() { return m_backend; }

private:
    NotificationService() = default;
    void LoadHistory();
    void SaveHistory();
    void TrimHistory();

    std::vector<Notification> m_notifications;
    mutable std::mutex m_mtx;

    BackendClient m_backend;
    NotifyEvent m_onPopup;
    ListEvent m_onListChanged;
    ListEvent m_onUnreadChanged;
    bool m_initialized = false;
};

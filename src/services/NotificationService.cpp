#include "pch.h"
#include "services/NotificationService.h"
#include "services/ConfigManager.h"

using namespace winrt;

NotificationService& NotificationService::Instance()
{
    static NotificationService inst;
    return inst;
}

void NotificationService::Init()
{
    if (m_initialized) return;
    m_initialized = true;

    LoadHistory();

    m_backend.SetOnNotify([this](const Notification& n) {
        AddNotification(n);
    });

    m_backend.Start();
}

void NotificationService::Shutdown()
{
    m_backend.Stop();
    SaveHistory();
}

void NotificationService::AddNotification(const Notification& n)
{
    if (!ShouldShow(n)) return;

    {
        std::lock_guard lock(m_mtx);
        m_notifications.insert(m_notifications.begin(), n);
        TrimHistory();
    }

    if (m_onPopup) m_onPopup(n);
    if (m_onListChanged) m_onListChanged();
    if (m_onUnreadChanged) m_onUnreadChanged();

    SaveHistory();
}

void NotificationService::RemoveNotification(const std::wstring& id)
{
    {
        std::lock_guard lock(m_mtx);
        std::erase_if(m_notifications, [&](auto& n) { return n.id == id; });
    }
    if (m_onListChanged) m_onListChanged();
    if (m_onUnreadChanged) m_onUnreadChanged();
    SaveHistory();
}

void NotificationService::ClearAll()
{
    {
        std::lock_guard lock(m_mtx);
        m_notifications.clear();
    }
    if (m_onListChanged) m_onListChanged();
    if (m_onUnreadChanged) m_onUnreadChanged();
    SaveHistory();
}

void NotificationService::MarkRead(const std::wstring& id)
{
    bool changed = false;
    {
        std::lock_guard lock(m_mtx);
        for (auto& n : m_notifications)
        {
            if (n.id == id && !n.read)
            {
                n.read = true;
                changed = true;
                break;
            }
        }
    }
    if (changed)
    {
        if (m_onListChanged) m_onListChanged();
        if (m_onUnreadChanged) m_onUnreadChanged();
        SaveHistory();
    }
}

void NotificationService::MarkAllRead()
{
    bool changed = false;
    {
        std::lock_guard lock(m_mtx);
        for (auto& n : m_notifications)
        {
            if (!n.read) { n.read = true; changed = true; }
        }
    }
    if (changed)
    {
        if (m_onListChanged) m_onListChanged();
        if (m_onUnreadChanged) m_onUnreadChanged();
        SaveHistory();
    }
}

std::vector<Notification> NotificationService::Search(const std::wstring& keyword,
    NotifyCategory cat, bool onlyUnread) const
{
    std::lock_guard lock(m_mtx);
    std::vector<Notification> result;

    for (auto& n : m_notifications)
    {
        if (onlyUnread && n.read) continue;
        if (static_cast<int>(cat) >= 0 && n.category != cat) continue;
        if (!keyword.empty())
        {
            auto titleLower = n.title;
            auto contentLower = n.content;
            auto kwLower = keyword;
            // 简单子串匹配（大小写不敏感需要转换，这里简化）
            if (titleLower.find(keyword) == std::wstring::npos &&
                contentLower.find(keyword) == std::wstring::npos)
                continue;
        }
        result.push_back(n);
    }
    return result;
}

int NotificationService::UnreadCount() const
{
    std::lock_guard lock(m_mtx);
    int cnt = 0;
    for (auto& n : m_notifications)
        if (!n.read) cnt++;
    return cnt;
}

std::vector<std::wstring> NotificationService::GetGroups() const
{
    std::lock_guard lock(m_mtx);
    std::vector<std::wstring> groups;
    std::unordered_map<std::wstring, bool> seen;

    for (auto& n : m_notifications)
    {
        if (!n.groupKey.empty() && !seen[n.groupKey])
        {
            seen[n.groupKey] = true;
            groups.push_back(n.groupKey);
        }
    }
    return groups;
}

bool NotificationService::ShouldShow(const Notification& n) const
{
    auto& cfg = ConfigManager::Instance().Get();
    for (auto& rule : cfg.filters)
    {
        bool match = true;
        if (!rule.category.empty() &&
            Notification::CategoryToString(n.category) != rule.category)
            match = false;
        if (!rule.priority.empty() &&
            Notification::PriorityToString(n.priority) != rule.priority)
            match = false;
        if (!rule.keyword.empty() &&
            n.title.find(rule.keyword) == std::wstring::npos &&
            n.content.find(rule.keyword) == std::wstring::npos)
            match = false;

        if (match && rule.block) return false;
    }
    return true;
}

void NotificationService::LoadHistory()
{
    try
    {
        auto localFolder = Windows::Storage::ApplicationData::Current().LocalFolder();
        auto file = localFolder.TryGetItemAsync(L"history.json").get();
        if (!file) return;

        auto storageFile = file.as<Windows::Storage::StorageFile>();
        auto text = Windows::Storage::FileIO::ReadTextAsync(storageFile).get();

        JsonObject obj;
        if (!JsonObject::Parse(text, obj)) return;

        auto arr = obj.GetNamedArray(L"notifications");
        std::lock_guard lock(m_mtx);
        m_notifications.clear();
        for (uint32_t i = 0; i < arr.Size(); i++)
        {
            m_notifications.push_back(Notification::FromJson(arr.GetAt(i).GetObjectW()));
        }
    }
    catch (...) {}
}

void NotificationService::SaveHistory()
{
    try
    {
        JsonArray arr;
        {
            std::lock_guard lock(m_mtx);
            for (auto& n : m_notifications)
                arr.Append(n.ToJson());
        }
        JsonObject obj;
        obj.SetNamedValue(L"notifications", arr);

        auto localFolder = Windows::Storage::ApplicationData::Current().LocalFolder();
        auto file = localFolder.CreateFileAsync(L"history.json",
            Windows::Storage::CreationCollisionOption::ReplaceExisting).get();
        Windows::Storage::FileIO::WriteTextAsync(file, obj.Stringify()).get();
    }
    catch (...) {}
}

void NotificationService::TrimHistory()
{
    auto& cfg = ConfigManager::Instance().Get();
    while ((int)m_notifications.size() > cfg.maxHistory)
        m_notifications.pop_back();
}

#include "NotifyCenter.h"
#include "Util.h"

using namespace winrt::Windows::Data::Json;

namespace winn
{
    NotifyCenter::NotifyCenter(Config& cfg) : m_cfg(cfg)
    {
        Load();
    }

    void NotifyCenter::Load()
    {
        std::string raw;
        if (!ReadFile(m_cfg.DataPath(), raw))
            return;
        try
        {
            JsonArray arr = JsonArray::Parse(winrt::to_hstring(raw));
            m_items.clear();
            for (auto&& v : arr)
            {
                auto it = NotifyItem::FromJson(v.GetObject());
                if (!it.id.empty())
                    m_items.push_back(std::move(it));
            }
        }
        catch (...) {}
    }

    void NotifyCenter::Save()
    {
        JsonArray arr;
        for (auto& it : m_items)
            arr.Append(it.ToJson());
        WriteFile(m_cfg.DataPath(), WToUtf8(std::wstring(arr.Stringify())));
    }

    void NotifyCenter::Add(const NotifyItem& it)
    {
        auto dup = std::find_if(m_items.begin(), m_items.end(), [&](const NotifyItem& x) { return x.id == it.id; });
        if (dup != m_items.end())
            m_items.erase(dup);
        m_items.insert(m_items.begin(), it);
        if ((int)m_items.size() > m_cfg.historyMax)
            m_items.resize(m_cfg.historyMax);
        Save();
        if (onChanged)
            onChanged();
    }

    bool NotifyCenter::Remove(const std::wstring& id)
    {
        auto dup = std::find_if(m_items.begin(), m_items.end(), [&](const NotifyItem& x) { return x.id == id; });
        if (dup == m_items.end())
            return false;
        m_items.erase(dup);
        Save();
        if (onChanged)
            onChanged();
        return true;
    }

    void NotifyCenter::Clear()
    {
        m_items.clear();
        Save();
        if (onChanged)
            onChanged();
    }

    void NotifyCenter::MarkRead(const std::wstring& id)
    {
        auto dup = std::find_if(m_items.begin(), m_items.end(), [&](const NotifyItem& x) { return x.id == id; });
        if (dup == m_items.end() || dup->read)
            return;
        dup->read = true;
        Save();
        if (onChanged)
            onChanged();
    }

    void NotifyCenter::MarkAllRead()
    {
        for (auto& it : m_items)
            it.read = true;
        Save();
        if (onChanged)
            onChanged();
    }

    uint32_t NotifyCenter::Unread() const
    {
        return (uint32_t)std::count_if(m_items.begin(), m_items.end(), [](const NotifyItem& x) { return !x.read; });
    }

    std::vector<NotifyItem> NotifyCenter::All() const
    {
        return m_items;
    }
}

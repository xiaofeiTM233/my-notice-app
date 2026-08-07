#pragma once
#include "pch.h"
#include "NotifyItem.h"
#include "Config.h"

namespace winn
{
    class NotifyCenter
    {
    public:
        explicit NotifyCenter(Config& cfg);

        void Add(const NotifyItem& it);
        bool Remove(const std::wstring& id);
        void Clear();
        void MarkRead(const std::wstring& id);
        void MarkAllRead();
        uint32_t Unread() const;
        std::vector<NotifyItem> All() const;
        void Load();
        void Save();

        std::function<void()> onChanged;

    private:
        std::vector<NotifyItem> m_items;
        Config& m_cfg;
    };
}

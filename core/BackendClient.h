#pragma once
#include "pch.h"
#include "NotifyItem.h"
#include "Config.h"

namespace winn
{
    class BackendClient
    {
    public:
        explicit BackendClient(Config& cfg);
        ~BackendClient();

        void Start();
        void Stop();
        void Pause(bool p);
        bool Paused() const { return m_paused; }

        std::function<void(const NotifyItem&)> onNotif;

    private:
        void WsLoop();
        void PollLoop();
        void HandleRaw(const std::string& utf8);
        void HandleObj(const winrt::Windows::Data::Json::JsonObject& o);

        Config& m_cfg;
        std::atomic<bool> m_run{ false };
        std::atomic<bool> m_paused{ false };
        std::thread m_thread;
        HANDLE m_stop = nullptr;
        std::wstring m_since;
    };
}

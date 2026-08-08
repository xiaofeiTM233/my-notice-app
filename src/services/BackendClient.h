#pragma once
#include "pch.h"
#include "models/Notification.h"

class BackendClient
{
public:
    using NotifyCallback = std::function<void(const Notification&)>;
    using StatusCallback = std::function<void(bool connected, const std::wstring& msg)>;

    BackendClient();
    ~BackendClient();

    void Start();
    void Stop();
    bool IsConnected() const { return m_connected.load(); }

    void SetOnNotify(NotifyCallback cb) { m_onNotify = std::move(cb); }
    void SetOnStatus(StatusCallback cb) { m_onStatus = std::move(cb); }

    void SendAck(const std::wstring& notifyId);

private:
    void RunWs();
    void RunHttpPoll();
    void ReconnectLoop();
    void ParseAndDispatch(const std::wstring& jsonStr);

    NotifyCallback m_onNotify;
    StatusCallback m_onStatus;
    std::atomic<bool> m_running{false};
    std::atomic<bool> m_connected{false};
    std::jthread m_workThread;
    std::jthread m_reconnectThread;

    // ws
    winrt::Windows::Networking::Sockets::MessageWebSocket m_ws{nullptr};
};

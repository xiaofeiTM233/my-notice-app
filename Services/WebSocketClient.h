#pragma once
#include "pch.h"

namespace nm {

using MsgHandler = std::function<void(const std::string&)>;
using StateHandler = std::function<void(bool connected)>;

class WebSocketClient {
public:
    WebSocketClient();
    ~WebSocketClient();

    void Connect(const std::string& url);
    void Disconnect();
    void Send(const std::string& msg);

    void OnMessage(MsgHandler h) { _onMsg = std::move(h); }
    void OnStateChange(StateHandler h) { _onState = std::move(h); }

    bool IsConnected() const { return _connected; }

private:
    void RunLoop(const std::string& url);
    void ReconnectLoop(const std::string& url);

    winrt::Windows::Networking::Sockets::MessageWebSocket _ws{ nullptr };
    std::atomic<bool> _connected{ false };
    std::atomic<bool> _running{ false };
    std::atomic<int> _reconnectAttempts{ 0 };
    int _maxReconnect = 10;
    int _reconnectDelay = 5;

    MsgHandler _onMsg;
    StateHandler _onState;
    std::mutex _mtx;

    std::thread _thread;
};

} // namespace nm

#include "pch.h"
#include "WebSocketClient.h"

namespace nm {

WebSocketClient::WebSocketClient() = default;

WebSocketClient::~WebSocketClient() {
    Disconnect();
}

void WebSocketClient::Connect(const std::string& url) {
    Disconnect();
    _running = true;
    _thread = std::thread(&WebSocketClient::RunLoop, this, url);
}

void WebSocketClient::Disconnect() {
    _running = false;
    try {
        if (_ws) {
            _ws.Close(1000, L"Client disconnecting");
            _ws = nullptr;
        }
    }
    catch (...) {}
    if (_thread.joinable()) _thread.join();
    _connected = false;
}

void WebSocketClient::Send(const std::string& msg) {
    if (!_connected || !_ws) return;
    try {
        winrt::Windows::Storage::Streams::DataWriter writer(_ws.OutputStream());
        writer.WriteString(winrt::to_hstring(msg));
        writer.StoreAsync().get();
    }
    catch (...) {}
}

void WebSocketClient::RunLoop(const std::string& url) {
    while (_running) {
        try {
            _ws = winrt::Windows::Networking::Sockets::MessageWebSocket();
            _ws.Control().MessageType(winrt::Windows::Networking::Sockets::SocketMessageType::Utf8);

            _ws.MessageReceived([this](auto&, auto& args) {
                try {
                    winrt::Windows::Storage::Streams::DataReader reader(args.GetDataReader());
                    reader.UnicodeEncoding(winrt::Windows::Storage::Streams::UnicodeEncoding::Utf8);
                    auto msg = winrt::to_string(reader.ReadString(reader.UnconsumedBufferLength()));
                    if (_onMsg) _onMsg(msg);
                }
                catch (...) {}
            });

            _ws.Closed([this, url](auto&, auto&) {
                bool wasConnected = _connected.exchange(false);
                if (_onState) _onState(false);
                if (_running && wasConnected) {
                    ReconnectLoop(url);
                }
            });

            auto uri = winrt::Windows::Foundation::Uri(winrt::to_hstring(url));
            _ws.ConnectAsync(uri).get();

            _connected = true;
            _reconnectAttempts = 0;
            if (_onState) _onState(true);

            // Block until closed or stop
            while (_running && _connected) {
                std::this_thread::sleep_for(std::chrono::milliseconds(500));
            }
        }
        catch (...) {
            bool wasConnected = _connected.exchange(false);
            if (_onState) _onState(false);
            _ws = nullptr;
            if (_running && wasConnected) {
                ReconnectLoop(url);
            }
        }
    }
}

void WebSocketClient::ReconnectLoop(const std::string& url) {
    while (_running && !_connected && _reconnectAttempts < _maxReconnect) {
        _reconnectAttempts++;
        std::this_thread::sleep_for(std::chrono::seconds(_reconnectDelay));
        if (!_running) break;
        try {
            _ws = winrt::Windows::Networking::Sockets::MessageWebSocket();
            _ws.Control().MessageType(winrt::Windows::Networking::Sockets::SocketMessageType::Utf8);

            _ws.MessageReceived([this](auto&, auto& args) {
                try {
                    winrt::Windows::Storage::Streams::DataReader reader(args.GetDataReader());
                    reader.UnicodeEncoding(winrt::Windows::Storage::Streams::UnicodeEncoding::Utf8);
                    auto msg = winrt::to_string(reader.ReadString(reader.UnconsumedBufferLength()));
                    if (_onMsg) _onMsg(msg);
                }
                catch (...) {}
            });

            _ws.Closed([this, url](auto&, auto&) {
                _connected = false;
                if (_onState) _onState(false);
            });

            auto uri = winrt::Windows::Foundation::Uri(winrt::to_hstring(url));
            _ws.ConnectAsync(uri).get();

            _connected = true;
            _reconnectAttempts = 0;
            if (_onState) _onState(true);
        }
        catch (...) {
            _ws = nullptr;
        }
    }
}

} // namespace nm

#include "pch.h"
#include "App.xaml.h"

using namespace winrt;
using namespace Microsoft::UI::Xaml;

// WinUI 3 应用入口
int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR, int)
{
    // 初始化 COM
    co_init_apartment();

    // 确保 Windows App SDK 运行时已初始化
    auto mddBootstrap = ::MddBootstrapInitialize2(
        0x00010005,  // Windows App SDK 1.5
        L"",
        MddBootstrapInitializeOptions_OnError_DebugBreak_IfDebuggerAttached);

    Application::Start([](auto&&) {
        make<WinNotify::implementation::App>();
    });

    ::MddBootstrapShutdown();
    return 0;
}

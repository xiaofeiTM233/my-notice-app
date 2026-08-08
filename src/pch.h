#pragma once

// Windows SDK
#include <windows.h>
#include <shellapi.h>
#include <commctrl.h>

// C++/WinRT
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Foundation.Collections.h>
#include <winrt/Windows.UI.Xaml.h>
#include <winrt/Windows.UI.Xaml.Controls.h>
#include <winrt/Windows.UI.Xaml.Hosting.h>
#include <winrt/Windows.UI.Xaml.Media.h>
#include <winrt/Windows.UI.Xaml.Media.Animation.h>
#include <winrt/Windows.UI.Xaml.Shapes.h>
#include <winrt/Windows.UI.Composition.h>
#include <winrt/Windows.UI.ViewManagement.h>
#include <winrt/Windows.Storage.h>
#include <winrt/Windows.Storage.Streams.h>
#include <winrt/Windows.Data.Json.h>
#include <winrt/Windows.Networking.Sockets.h>
#include <winrt/Windows.Networking.Connectivity.h>
#include <winrt/Windows.Web.Http.h>
#include <winrt/Windows.System.h>
#include <winrt/Microsoft.UI.Xaml.h>
#include <winrt/Microsoft.UI.Xaml.Controls.h>
#include <winrt/Microsoft.UI.Xaml.Media.h>
#include <winrt/Microsoft.UI.Xaml.Media.Animation.h>
#include <winrt/Microsoft.UI.Xaml.Shapes.h>
#include <winrt/Microsoft.UI.Windowing.h>
#include <winrt/Microsoft.UI.Composition.h>
#include <winrt/Microsoft.UI.Interop.h>

// STL
#include <string>
#include <vector>
#include <memory>
#include <functional>
#include <mutex>
#include <atomic>
#include <chrono>
#include <unordered_map>
#include <algorithm>
#include <fstream>
#include <sstream>
#include <format>

namespace winrt
{
    using namespace Windows::Foundation;
    using namespace Windows::Foundation::Collections;
    using namespace Windows::UI::Xaml;
    using namespace Windows::UI::Xaml::Controls;
    using namespace Windows::UI::Xaml::Media;
    using namespace Windows::UI::Xaml::Media::Animation;
    using namespace Windows::UI::Composition;
    using namespace Windows::Storage;
    using namespace Windows::Data::Json;
    using namespace Windows::Networking::Sockets;
    using namespace Windows::Web::Http;
    using namespace Windows::System;

    namespace mux = Microsoft::UI::Xaml;
    namespace muxc = Microsoft::UI::Xaml::Controls;
    namespace muxm = Microsoft::UI::Xaml::Media;
    namespace muxa = Microsoft::UI::Xaml::Media::Animation;
}

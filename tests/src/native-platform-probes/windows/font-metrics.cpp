#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <ole2.h>
#include <UIAutomation.h>
#include <dwrite_3.h>
#include <wrl/client.h>
#include <algorithm>
#include <cerrno>
#include <cmath>
#include <cwchar>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <stdexcept>
#include <string>
using Microsoft::WRL::ComPtr;

// SDK-only acceptance diagnostic for the bundled-font fixture. A parent process
// must enforce a deadline because UIA calls cross into another native provider.
static void check(HRESULT hr, char const* stage) {
    std::cout << "{\"checkpoint\":\"" << stage << "\",\"hresult\":" << static_cast<long>(hr) << "}" << std::endl;
    if (FAILED(hr)) throw std::runtime_error(stage);
}
static std::string quote(wchar_t const* value, size_t length) {
    std::ostringstream out; out << '"';
    for (size_t i = 0; i < length; ++i) {
        unsigned c = static_cast<unsigned short>(value[i]);
        if (c == '"' || c == '\\') out << '\\' << static_cast<char>(c);
        else if (c >= 32 && c < 127) out << static_cast<char>(c);
        else out << "\\u" << std::hex << std::setw(4) << std::setfill('0') << c << std::dec;
    }
    out << '"'; return out.str();
}
struct Bstr { BSTR value = nullptr; ~Bstr() { SysFreeString(value); } };
struct Array { SAFEARRAY* value = nullptr; ~Array() { if (value) SafeArrayDestroy(value); } };
struct Variant { VARIANT value; Variant() { VariantInit(&value); } ~Variant() { VariantClear(&value); } };
struct Search { DWORD pid; HWND window = nullptr; };
static BOOL CALLBACK find_window(HWND window, LPARAM data) {
    auto& search = *reinterpret_cast<Search*>(data); DWORD pid = 0;
    GetWindowThreadProcessId(window, &pid);
    if (pid == search.pid && IsWindowVisible(window) && GetWindow(window, GW_OWNER) == nullptr) { search.window = window; return FALSE; }
    return TRUE;
}
static float measure(IDWriteFactory5* factory, IDWriteFontCollection* fonts, wchar_t const* family, std::wstring const& text) {
    ComPtr<IDWriteTextFormat> format;
    check(factory->CreateTextFormat(family, fonts, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL,
        DWRITE_FONT_STRETCH_NORMAL, 18.0f, L"en-US", &format), "CreateTextFormat");
    ComPtr<IDWriteTextLayout> layout;
    check(factory->CreateTextLayout(text.c_str(), static_cast<UINT32>(text.size()), format.Get(), 100000, 1000, &layout), "CreateTextLayout");
    DWRITE_TEXT_METRICS metrics{}; check(layout->GetMetrics(&metrics), "GetMetrics");
    return metrics.widthIncludingTrailingWhitespace;
}
static int inspect(DWORD pid, wchar_t const* automation_id, wchar_t const* font_file) {
    Search search{pid}; EnumWindows(find_window, reinterpret_cast<LPARAM>(&search));
    if (!search.window) throw std::runtime_error("Target window absent");
    auto dpi = GetDpiForWindow(search.window);
    if (!dpi) throw std::runtime_error("Window DPI absent");
    ComPtr<IUIAutomation> automation;
    check(CoCreateInstance(__uuidof(CUIAutomation), nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&automation)), "CUIAutomation");
    ComPtr<IUIAutomation2> newer;
    if (SUCCEEDED(automation.As(&newer))) { check(newer->put_ConnectionTimeout(5000), "ConnectionTimeout"); check(newer->put_TransactionTimeout(5000), "TransactionTimeout"); }
    ComPtr<IUIAutomationElement> root, label;
    check(automation->ElementFromHandle(search.window, &root), "ElementFromHandle");
    Variant id; id.value.vt = VT_BSTR; id.value.bstrVal = SysAllocString(automation_id);
    if (!id.value.bstrVal) throw std::bad_alloc();
    ComPtr<IUIAutomationCondition> condition;
    check(automation->CreatePropertyCondition(UIA_AutomationIdPropertyId, id.value, &condition), "AutomationIdCondition");
    check(root->FindFirst(TreeScope_Descendants, condition.Get(), &label), "FindFontLabel");
    if (!label) throw std::runtime_error("Font label absent");
    int actual_pid = 0; check(label->get_CurrentProcessId(&actual_pid), "LabelProcessId");
    if (actual_pid != static_cast<int>(pid)) throw std::runtime_error("Wrong label process");
    ComPtr<IUIAutomationTextPattern> pattern;
    check(label->GetCurrentPatternAs(UIA_TextPatternId, IID_PPV_ARGS(&pattern)), "FontTextPattern");
    ComPtr<IUIAutomationTextRange> document;
    check(pattern->get_DocumentRange(&document), "FontDocumentRange");
    Bstr text; check(document->GetText(-1, &text.value), "FontDocumentGetText");
    std::wstring content(text.value, SysStringLen(text.value));
    if (content != L"Bundled Manrope: WWW iii 012345") throw std::runtime_error("Unexpected font fixture content");
    Array rectangles; check(document->GetBoundingRectangles(&rectangles.value), "FontGlyphRectangles");
    if (!rectangles.value || SafeArrayGetDim(rectangles.value) != 1) throw std::runtime_error("Missing text rectangles");
    LONG lower = 0, upper = -1;
    check(SafeArrayGetLBound(rectangles.value, 1, &lower), "RectangleLowerBound");
    check(SafeArrayGetUBound(rectangles.value, 1, &upper), "RectangleUpperBound");
    if (upper - lower + 1 != 4) throw std::runtime_error("Font fixture must expose one unclipped line");
    double physical_width = 0; LONG width_index = lower + 2;
    check(SafeArrayGetElement(rectangles.value, &width_index, &physical_width), "GlyphLineWidth");
    double observed = physical_width * 96.0 / dpi;
    if (!std::isfinite(observed) || observed <= 0) throw std::runtime_error("Invalid glyph width");
    Variant name; check(document->GetAttributeValue(UIA_FontNameAttributeId, &name.value), "ReportedFontName");
    ComPtr<IDWriteFactory5> factory;
    check(DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, __uuidof(IDWriteFactory5), reinterpret_cast<IUnknown**>(factory.GetAddressOf())), "DWriteFactory5");
    ComPtr<IDWriteFontSetBuilder1> builder;
    check(factory->CreateFontSetBuilder(&builder), "CreateFontSetBuilder");
    ComPtr<IDWriteFontFile> file;
    check(factory->CreateFontFileReference(font_file, nullptr, &file), "CreateFontFileReference");
    check(builder->AddFontFile(file.Get()), "AddExactStagedFontFile");
    ComPtr<IDWriteFontSet> set; check(builder->CreateFontSet(&set), "CreateFontSet");
    ComPtr<IDWriteFontCollection1> collection;
    check(factory->CreateFontCollectionFromFontSet(set.Get(), &collection), "CreateFontCollectionFromFontSet");
    UINT32 index = 0; BOOL exists = FALSE;
    check(collection->FindFamilyName(L"Manrope", &index, &exists), "FindBundledFamily");
    if (!exists) throw std::runtime_error("Staged file does not contain Manrope");
    auto expected = measure(factory.Get(), collection.Get(), L"Manrope", content);
    auto arial = measure(factory.Get(), nullptr, L"Arial", content);
    auto segoe = measure(factory.Get(), nullptr, L"Segoe UI", content);
    constexpr double tolerance = 1.5;
    bool distinct = std::abs(expected - arial) > tolerance * 2 && std::abs(expected - segoe) > tolerance * 2;
    bool matches = std::abs(observed - expected) <= tolerance;
    bool excludes_fallback = std::abs(observed - arial) > tolerance && std::abs(observed - segoe) > tolerance;
    std::cout << std::setprecision(10) << "{\"stage\":\"complete\",\"pid\":" << pid
        << ",\"hwnd\":" << reinterpret_cast<uintptr_t>(search.window) << ",\"automationId\":" << quote(automation_id, std::wcslen(automation_id))
        << ",\"dpi\":" << dpi << ",\"text\":" << quote(content.c_str(), content.size())
        << ",\"fontFile\":" << quote(font_file, std::wcslen(font_file))
        << ",\"reportedFontName\":" << (name.value.vt == VT_BSTR ? quote(name.value.bstrVal, SysStringLen(name.value.bstrVal)) : "null")
        << ",\"observedWidthDip\":" << observed << ",\"manropeWidthDip\":" << expected
        << ",\"arialWidthDip\":" << arial << ",\"segoeWidthDip\":" << segoe << ",\"toleranceDip\":" << tolerance
        << ",\"negativeControlsDistinct\":" << (distinct ? "true" : "false")
        << ",\"qualified\":" << (matches && distinct && excludes_fallback ? "true" : "false") << "}" << std::endl;
    return matches && distinct && excludes_fallback ? 0 : 1;
}
int wmain(int argc, wchar_t** argv) {
    if (argc != 4) { std::cerr << "Usage: font-metrics.exe PID AutomationId StagedManropeFontFile\n"; return 2; }
    wchar_t* end = nullptr; errno = 0; unsigned long pid = std::wcstoul(argv[1], &end, 10);
    if (errno || *end || !pid) return 2;
    if (!SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)) return 2;
    HRESULT hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(hr)) return 2;
    int result = 2;
    try { result = inspect(pid, argv[2], argv[3]); }
    catch (std::exception const& error) { std::cerr << "Font metric gate failed: " << error.what() << '\n'; }
    CoUninitialize(); return result;
}

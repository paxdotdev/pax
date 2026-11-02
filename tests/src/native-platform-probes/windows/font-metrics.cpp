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
#include <vector>
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
struct FontSpec {
    std::wstring family = L"Manrope";
    float size = 18;
    DWRITE_FONT_WEIGHT weight = DWRITE_FONT_WEIGHT_NORMAL;
    DWRITE_FONT_STYLE style = DWRITE_FONT_STYLE_NORMAL;
    std::wstring expected = L"Bundled Manrope: @@@@@@@@";
    std::wstring segment = L"@@@@@@@@";
};
static ComPtr<IDWriteTextLayout> layout_for(IDWriteFactory5* factory, IDWriteFontCollection* fonts, wchar_t const* family, std::wstring const& text, FontSpec const& spec) {
    ComPtr<IDWriteTextFormat> format;
    check(factory->CreateTextFormat(family, fonts, spec.weight, spec.style,
        DWRITE_FONT_STRETCH_NORMAL, spec.size, L"en-US", &format), "CreateTextFormat");
    ComPtr<IDWriteTextLayout> layout;
    check(factory->CreateTextLayout(text.c_str(), static_cast<UINT32>(text.size()), format.Get(), 100000, 1000, &layout), "CreateTextLayout");
    return layout;
}
static double native_width(IUIAutomationTextRange* range, UINT dpi) {
    Array rectangles; check(range->GetBoundingRectangles(&rectangles.value), "FontGlyphRectangles");
    if (!rectangles.value || SafeArrayGetDim(rectangles.value) != 1) throw std::runtime_error("Missing text rectangles");
    LONG lower = 0, upper = -1;
    check(SafeArrayGetLBound(rectangles.value, 1, &lower), "RectangleLowerBound");
    check(SafeArrayGetUBound(rectangles.value, 1, &upper), "RectangleUpperBound");
    if (upper - lower + 1 != 4) throw std::runtime_error("Font fixture range must expose one unclipped line");
    double physical_width = 0; LONG width_index = lower + 2;
    check(SafeArrayGetElement(rectangles.value, &width_index, &physical_width), "GlyphLineWidth");
    double observed = physical_width * 96.0 / dpi;
    if (!std::isfinite(observed) || observed <= 0) throw std::runtime_error("Invalid glyph width");
    return observed;
}
static double layout_width(IDWriteTextLayout* layout, UINT32 start, UINT32 length, bool full) {
    if (full) {
        DWRITE_TEXT_METRICS metrics{}; check(layout->GetMetrics(&metrics), "GetMetrics");
        return metrics.widthIncludingTrailingWhitespace;
    }
    // Shape once in the complete original string. Measuring an isolated word
    // could change kerning/ligature context relative to the native range.
    DWRITE_HIT_TEST_METRICS range{}; UINT32 count = 0;
    check(layout->HitTestTextRange(start, length, 0, 0, &range, 1, &count), "HitTestTextRange");
    if (count != 1) throw std::runtime_error("Diagnostic DirectWrite range must occupy one line/run");
    return range.width;
}
static ComPtr<IDWriteFontCollection1> staged_collection(IDWriteFactory5* factory, wchar_t const* font_file, FontSpec const& spec) {
    ComPtr<IDWriteFontSetBuilder1> builder;
    check(factory->CreateFontSetBuilder(&builder), "CreateFontSetBuilder");
    ComPtr<IDWriteFontFile> file;
    check(factory->CreateFontFileReference(font_file, nullptr, &file), "CreateFontFileReference");
    check(builder->AddFontFile(file.Get()), "AddExactStagedFontFile");
    ComPtr<IDWriteFontSet> set; check(builder->CreateFontSet(&set), "CreateFontSet");
    ComPtr<IDWriteFontCollection1> collection;
    check(factory->CreateFontCollectionFromFontSet(set.Get(), &collection), "CreateFontCollectionFromFontSet");
    UINT32 index = 0; BOOL exists = FALSE;
    check(collection->FindFamilyName(spec.family.c_str(), &index, &exists), "FindBundledFamily");
    if (!exists) throw std::runtime_error("Staged file does not contain requested family");
    return collection;
}
static int inspect(DWORD pid, wchar_t const* automation_id, wchar_t const* font_file, FontSpec const& spec) {
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
    if (content != spec.expected) throw std::runtime_error("Unexpected font fixture content");
    double observed = native_width(document.Get(), dpi);
    Variant name; check(document->GetAttributeValue(UIA_FontNameAttributeId, &name.value), "ReportedFontName");
    ComPtr<IDWriteFactory5> factory;
    check(DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, __uuidof(IDWriteFactory5), reinterpret_cast<IUnknown**>(factory.GetAddressOf())), "DWriteFactory5");
    ComPtr<IDWriteFactory> system_factory; check(factory.As(&system_factory), "SystemFactory");
    ComPtr<IDWriteFontCollection> system_fonts;
    check(system_factory->GetSystemFontCollection(&system_fonts), "SystemFontCollection");
    UINT32 system_index = 0; BOOL system_family = FALSE;
    check(system_fonts->FindFamilyName(spec.family.c_str(), &system_index, &system_family), "FindSystemFamily");
    auto collection = staged_collection(factory.Get(), font_file, spec);
    auto bundled_layout = layout_for(factory.Get(), collection.Get(), spec.family.c_str(), content, spec);
    auto arial_layout = layout_for(factory.Get(), nullptr, L"Arial", content, spec);
    auto segoe_layout = layout_for(factory.Get(), nullptr, L"Segoe UI", content, spec);
    struct Measurement { std::wstring text; double observed, bundled, arial, segoe; };
    std::vector<Measurement> ranges;
    ranges.push_back({content, observed,
        layout_width(bundled_layout.Get(), 0, UINT32(content.size()), true),
        layout_width(arial_layout.Get(), 0, UINT32(content.size()), true),
        layout_width(segoe_layout.Get(), 0, UINT32(content.size()), true)});
    // Repeated @ glyphs differ clearly from both fallback families at this
    // font size. Retain a separate range so prefix advances cannot conceal
    // an ambiguous negative control.
    if (!spec.segment.empty()) {
        auto const& segment_text = spec.segment;
        auto position = content.find(segment_text);
        if (position == std::wstring::npos || content.find(segment_text, position + 1) != std::wstring::npos)
            throw std::runtime_error("Ambiguous discriminating fixture range");
        auto start = UINT32(position), length = UINT32(segment_text.size());
        ComPtr<IUIAutomationTextRange> range;
        // WinUI's native TextBlock provider can reject FindText. Its endpoint
        // operations expose the same exact ASCII range without a text search.
        check(document->Clone(&range), "CloneDiscriminatingTextRange");
        int moved = 0;
        if (start + length == content.size()) {
            // UIA Character units may follow text clusters, not UTF-16 code
            // units. The diagnostic suffix is ASCII: count back from its
            // known end without crossing ligatures in the preceding text.
            check(range->MoveEndpointByRange(TextPatternRangeEndpoint_Start, document.Get(), TextPatternRangeEndpoint_End),
                "CollapseDiscriminatingRangeToDocumentEnd");
            check(range->MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Character, -int(length), &moved),
                "RetreatDiscriminatingRangeStart");
            if (moved != -int(length)) throw std::runtime_error("Native range start did not reach suffix");
        } else {
            check(range->MoveEndpointByRange(TextPatternRangeEndpoint_End, document.Get(), TextPatternRangeEndpoint_Start),
                "CollapseDiscriminatingRangeToDocumentStart");
            check(range->MoveEndpointByUnit(TextPatternRangeEndpoint_End, TextUnit_Character, int(start + length), &moved),
                "AdvanceDiscriminatingRangeEnd");
            if (moved != int(start + length)) throw std::runtime_error("Native range end did not reach requested character");
            check(range->MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Character, int(start), &moved),
                "AdvanceDiscriminatingRangeStart");
            if (moved != int(start)) throw std::runtime_error("Native range start did not reach requested character");
        }
        Bstr actual; check(range->GetText(-1, &actual.value), "DiscriminatingRangeGetText");
        if (std::wstring(actual.value, SysStringLen(actual.value)) != segment_text)
            throw std::runtime_error("Wrong discriminating native range");
        ranges.push_back({segment_text, native_width(range.Get(), dpi),
            layout_width(bundled_layout.Get(), start, length, false),
            layout_width(arial_layout.Get(), start, length, false),
            layout_width(segoe_layout.Get(), start, length, false)});
    }
    auto expected = ranges.front().bundled, arial = ranges.front().arial, segoe = ranges.front().segoe;
    constexpr double tolerance = 1.5;
    bool matches = true, distinct_arial = false, distinct_segoe = false;
    bool excludes_arial = false, excludes_segoe = false;
    for (auto const& range : ranges) {
        matches &= std::abs(range.observed - range.bundled) <= tolerance;
        bool separates_arial = std::abs(range.bundled - range.arial) > tolerance * 2;
        bool separates_segoe = std::abs(range.bundled - range.segoe) > tolerance * 2;
        distinct_arial |= separates_arial; distinct_segoe |= separates_segoe;
        excludes_arial |= separates_arial && std::abs(range.observed - range.arial) > tolerance;
        excludes_segoe |= separates_segoe && std::abs(range.observed - range.segoe) > tolerance;
    }
    bool distinct = distinct_arial && distinct_segoe;
    bool excludes_fallback = excludes_arial && excludes_segoe;
    std::cout << std::setprecision(10) << "{\"stage\":\"complete\",\"pid\":" << pid
        << ",\"hwnd\":" << reinterpret_cast<uintptr_t>(search.window) << ",\"automationId\":" << quote(automation_id, std::wcslen(automation_id))
        << ",\"dpi\":" << dpi << ",\"text\":" << quote(content.c_str(), content.size())
        << ",\"fontFile\":" << quote(font_file, std::wcslen(font_file))
        << ",\"family\":" << quote(spec.family.c_str(), spec.family.size())
        << ",\"fontSize\":" << spec.size << ",\"fontWeight\":" << int(spec.weight)
        << ",\"italic\":" << (spec.style == DWRITE_FONT_STYLE_ITALIC ? "true" : "false")
        << ",\"systemFamilyPresent\":" << (system_family ? "true" : "false")
        << ",\"reportedFontName\":" << (name.value.vt == VT_BSTR ? quote(name.value.bstrVal, SysStringLen(name.value.bstrVal)) : "null")
        << ",\"observedWidthDip\":" << observed << ",\"bundledWidthDip\":" << expected
        << ",\"arialWidthDip\":" << arial << ",\"segoeWidthDip\":" << segoe << ",\"toleranceDip\":" << tolerance
        << ",\"negativeControlsDistinct\":" << (distinct ? "true" : "false")
        << ",\"qualified\":" << (matches && distinct && excludes_fallback ? "true" : "false")
        << ",\"allRangesMatchBundled\":" << (matches ? "true" : "false")
        << ",\"excludesArial\":" << (excludes_arial ? "true" : "false")
        << ",\"excludesSegoeUI\":" << (excludes_segoe ? "true" : "false")
        ;
    // Retain the original Manrope diagnostic fields for existing consumers.
    if (spec.family == L"Manrope") std::cout << ",\"manropeWidthDip\":" << expected
        << ",\"allRangesMatchManrope\":" << (matches ? "true" : "false");
    std::cout << ",\"ranges\":[";
    for (size_t i = 0; i < ranges.size(); ++i) {
        auto const& range = ranges[i];
        if (i) std::cout << ',';
        std::cout << "{\"text\":" << quote(range.text.c_str(), range.text.size())
            << ",\"observedWidthDip\":" << range.observed << ",\"bundledWidthDip\":" << range.bundled
            << ",\"arialWidthDip\":" << range.arial << ",\"segoeWidthDip\":" << range.segoe;
        if (spec.family == L"Manrope") std::cout << ",\"manropeWidthDip\":" << range.bundled;
        std::cout << '}';
    }
    std::cout << "]}" << std::endl;
    return matches && distinct && excludes_fallback ? 0 : 1;
}
int wmain(int argc, wchar_t** argv) {
    if (argc != 4 && argc != 10) { std::cerr << "Usage: font-metrics.exe PID AutomationId FontFile [Family SizePx Weight Italic01 ExactText DiscriminatingText]\n"; return 2; }
    FontSpec spec;
    if (argc == 10) {
        wchar_t* end = nullptr; errno = 0;
        spec.family = argv[4]; spec.size = std::wcstof(argv[5], &end);
        if (errno || *end || !std::isfinite(spec.size) || spec.size <= 0) return 2;
        auto weight = std::wcstol(argv[6], &end, 10);
        if (errno || *end || weight < 1 || weight > 999) return 2;
        spec.weight = static_cast<DWRITE_FONT_WEIGHT>(weight);
        if (std::wcscmp(argv[7], L"0") && std::wcscmp(argv[7], L"1")) return 2;
        spec.style = argv[7][0] == L'1' ? DWRITE_FONT_STYLE_ITALIC : DWRITE_FONT_STYLE_NORMAL;
        spec.expected = argv[8]; spec.segment = argv[9];
        if (spec.family.empty() || spec.expected.empty()) return 2;
    }
    wchar_t* end = nullptr; errno = 0; unsigned long pid = std::wcstoul(argv[1], &end, 10);
    if (errno || *end || !pid) return 2;
    if (!SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)) return 2;
    HRESULT hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(hr)) return 2;
    int result = 2;
    try { result = inspect(pid, argv[2], argv[3], spec); }
    catch (std::exception const& error) { std::cerr << "Font metric gate failed: " << error.what() << '\n'; }
    CoUninitialize(); return result;
}

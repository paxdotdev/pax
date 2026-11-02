#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <ole2.h>
#include <UIAutomation.h>
#include <wrl/client.h>
#include <algorithm>
#include <cerrno>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <cwchar>
#include <cwctype>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

using Microsoft::WRL::ComPtr;

// This probe never sets focus, edits text, changes input profiles, or injects
// keys. UIA provider calls can hang despite optional timeouts: the caller must
// supervise the process with a deadline and retain its flushed checkpoints.
namespace {
constexpr int TextLimit = 4096;
constexpr size_t WindowLimit = 128;
struct Bstr { BSTR value = nullptr; ~Bstr() { SysFreeString(value); } };
struct Array { SAFEARRAY* value = nullptr; ~Array() { if (value) SafeArrayDestroy(value); } };

std::string quoted(wchar_t const* value, size_t length) {
    std::ostringstream out; out << '"';
    for (size_t i = 0; i < length; ++i) {
        auto ch = static_cast<unsigned short>(value[i]);
        if (ch == '"' || ch == '\\') out << '\\' << static_cast<char>(ch);
        else if (ch >= 32 && ch < 127) out << static_cast<char>(ch);
        else out << "\\u" << std::hex << std::setfill('0') << std::setw(4) << ch << std::dec;
    }
    out << '"'; return out.str();
}
std::string quoted(BSTR value) { return quoted(value, SysStringLen(value)); }
std::string rect(RECT value) {
    return "[" + std::to_string(value.left) + "," + std::to_string(value.top) + "," +
        std::to_string(value.right - value.left) + "," + std::to_string(value.bottom - value.top) + "]";
}
bool emit(std::string const& stage, HRESULT hr, std::string const& details = "") {
    std::ostringstream code; code << "0x" << std::hex << std::setfill('0') << std::setw(8) << static_cast<unsigned long>(hr);
    std::cout << "{\"stage\":" << quoted(std::wstring(stage.begin(), stage.end()).c_str(), stage.size())
        << ",\"hresult\":\"" << code.str() << "\",\"succeeded\":" << (SUCCEEDED(hr) ? "true" : "false")
        << details << "}" << std::endl;
    return SUCCEEDED(hr);
}
void checkpoint(std::string const& stage) { emit("checkpoint", S_OK, ",\"next\":\"" + stage + "\""); }

void element(IUIAutomationElement* value, std::string const& stage, int index = -1, int depth = 0) {
    if (!value) { emit(stage, E_POINTER); return; }
    Bstr name, id, class_name; RECT bounds{}; int pid = 0, type = 0; BOOL focus = FALSE, offscreen = FALSE;
    HRESULT a = value->get_CurrentName(&name.value), b = value->get_CurrentAutomationId(&id.value);
    HRESULT c = value->get_CurrentClassName(&class_name.value), d = value->get_CurrentBoundingRectangle(&bounds);
    HRESULT e = value->get_CurrentProcessId(&pid), f = value->get_CurrentHasKeyboardFocus(&focus);
    HRESULT g = value->get_CurrentIsOffscreen(&offscreen), h = value->get_CurrentControlType(&type);
    HRESULT hr = S_OK; for (auto status : {a, b, c, d, e, f, g, h}) if (FAILED(status)) { hr = status; break; }
    emit(stage, hr, ",\"index\":" + std::to_string(index) + ",\"depth\":" + std::to_string(depth) +
        ",\"pid\":" + std::to_string(pid) + ",\"name\":" + quoted(name.value) + ",\"automationId\":" + quoted(id.value) +
        ",\"className\":" + quoted(class_name.value) + ",\"controlType\":" + std::to_string(type) +
        ",\"hasKeyboardFocus\":" + (focus ? "true" : "false") + ",\"offscreen\":" + (offscreen ? "true" : "false") +
        ",\"boundsPhysical\":" + rect(bounds));
}

void range(IUIAutomationTextRange* value, std::string const& stage) {
    emit(stage + ".present", S_OK, std::string(",\"present\":") + (value ? "true" : "false"));
    if (!value) return;
    checkpoint(stage + ".GetText");
    Bstr text; auto hr = value->GetText(TextLimit, &text.value);
    emit(stage + ".text", hr, SUCCEEDED(hr) ? ",\"text\":" + quoted(text.value) + ",\"limit\":4096" : "");
    int order = 0; hr = value->CompareEndpoints(TextPatternRangeEndpoint_Start, value, TextPatternRangeEndpoint_End, &order);
    emit(stage + ".endpoints", hr, ",\"startToEnd\":" + std::to_string(order));
    checkpoint(stage + ".GetBoundingRectangles");
    Array boxes; hr = value->GetBoundingRectangles(&boxes.value);
    if (!emit(stage + ".boundingRectangles", hr) || !boxes.value) return;
    LONG lower = 0, upper = -1; VARTYPE type = VT_EMPTY;
    if (SafeArrayGetDim(boxes.value) != 1 || FAILED(SafeArrayGetVartype(boxes.value, &type)) || type != VT_R8 ||
        FAILED(SafeArrayGetLBound(boxes.value, 1, &lower)) || FAILED(SafeArrayGetUBound(boxes.value, 1, &upper)) ||
        upper - lower + 1 < 0 || (upper - lower + 1) % 4 != 0 || upper - lower + 1 > 256) {
        emit(stage + ".rectanglesPhysical", E_UNEXPECTED); return;
    }
    std::ostringstream out; out << std::setprecision(15) << ",\"rectangles\":[";
    for (LONG i = lower; i <= upper; ++i) {
        double coordinate = 0; hr = SafeArrayGetElement(boxes.value, &i, &coordinate);
        if (FAILED(hr) || !std::isfinite(coordinate)) {
            emit(stage + ".rectanglesPhysical", FAILED(hr) ? hr : E_UNEXPECTED); return;
        }
        if (i != lower) out << ',';
        out << coordinate;
    }
    out << "]"; emit(stage + ".rectanglesPhysical", S_OK, out.str());
}

void terminal_range(IUIAutomationTextRange* document) {
    constexpr int TerminalUnitLimit = 16;
    if (!document) { emit("document.terminal.selection", E_POINTER); return; }
    ComPtr<IUIAutomationTextRange> terminal;
    checkpoint("document.terminal.Clone");
    auto hr = document->Clone(&terminal);
    emit("document.terminal.Clone", hr);
    if (hr != S_OK || !terminal) {
        if (hr == S_OK) emit("document.terminal.selection", E_POINTER);
        return;
    }
    checkpoint("document.terminal.CollapseToEnd");
    hr = terminal->MoveEndpointByRange(TextPatternRangeEndpoint_Start, document, TextPatternRangeEndpoint_End);
    emit("document.terminal.CollapseToEnd", hr);
    if (hr != S_OK) return;
    // Document rectangles omit clipped lines. Observe the final non-line-break
    // unit separately, allowing a bounded number of provider-added CR/LF units.
    for (int skipped = 0; skipped < TerminalUnitLimit; ++skipped) {
        int moved = 0;
        checkpoint("document.terminal.MoveStart");
        hr = terminal->MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Character, -1, &moved);
        emit("document.terminal.MoveStart", hr, ",\"attempt\":" + std::to_string(skipped + 1) +
            ",\"moved\":" + std::to_string(moved));
        if (hr != S_OK) return;
        if (moved != -1) {
            emit("document.terminal.selection", S_FALSE, ",\"nonWhitespace\":false,\"reason\":\"no_previous_character\"");
            range(terminal.Get(), "document.terminal"); return;
        }
        Bstr unit;
        checkpoint("document.terminal.ProbeText");
        hr = terminal->GetText(TextLimit, &unit.value);
        emit("document.terminal.ProbeText", hr, SUCCEEDED(hr) ? ",\"text\":" + quoted(unit.value) : "");
        if (hr != S_OK) return;
        auto length = SysStringLen(unit.value);
        bool line_break_only = length != 0, non_whitespace = false;
        for (UINT i = 0; i < length; ++i) {
            line_break_only = line_break_only && (unit.value[i] == L'\r' || unit.value[i] == L'\n');
            non_whitespace = non_whitespace || !std::iswspace(unit.value[i]);
        }
        if (!line_break_only) {
            emit("document.terminal.selection", non_whitespace ? S_OK : S_FALSE,
                ",\"skippedLineBreakUnits\":" + std::to_string(skipped) +
                ",\"nonWhitespace\":" + (non_whitespace ? "true" : "false"));
            range(terminal.Get(), "document.terminal"); return;
        }
        if (skipped + 1 == TerminalUnitLimit) {
            emit("document.terminal.selection", S_FALSE, ",\"nonWhitespace\":false,\"reason\":\"line_break_limit\",\"skippedLineBreakUnits\":16");
            range(terminal.Get(), "document.terminal"); return;
        }
        checkpoint("document.terminal.RetreatEnd");
        hr = terminal->MoveEndpointByRange(TextPatternRangeEndpoint_End, terminal.Get(), TextPatternRangeEndpoint_Start);
        emit("document.terminal.RetreatEnd", hr);
        if (hr != S_OK) return;
    }
}

struct Windows { std::vector<HWND> values; bool truncated = false; bool visible_only = true; };
BOOL CALLBACK collect_window(HWND window, LPARAM data) {
    auto& windows = *reinterpret_cast<Windows*>(data);
    if (windows.visible_only && !IsWindowVisible(window)) return TRUE;
    if (windows.values.size() == WindowLimit) { windows.truncated = true; return FALSE; }
    windows.values.push_back(window); return TRUE;
}
void window(HWND handle, char const* stage) {
    DWORD pid = 0; auto thread = GetWindowThreadProcessId(handle, &pid); RECT bounds{};
    auto ok = GetWindowRect(handle, &bounds);
    auto bounds_status = ok ? S_OK : HRESULT_FROM_WIN32(GetLastError());
    wchar_t name[512]{}, class_name[256]{}, path[32768]{};
    GetWindowTextW(handle, name, 512); GetClassNameW(handle, class_name, 256);
    HANDLE process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
    if (process) { DWORD length = 32768; if (!QueryFullProcessImageNameW(process, 0, path, &length)) path[0] = 0; CloseHandle(process); }
    auto layout = GetKeyboardLayout(thread);
    emit(stage, bounds_status, ",\"hwnd\":" + std::to_string(reinterpret_cast<uintptr_t>(handle)) +
        ",\"pid\":" + std::to_string(pid) + ",\"threadId\":" + std::to_string(thread) +
        ",\"ownerHwnd\":" + std::to_string(reinterpret_cast<uintptr_t>(GetWindow(handle, GW_OWNER))) +
        ",\"name\":" + quoted(name, std::wcslen(name)) + ",\"className\":" + quoted(class_name, std::wcslen(class_name)) +
        ",\"processImage\":" + quoted(path, std::wcslen(path)) + ",\"boundsPhysical\":" + rect(bounds) +
        ",\"visible\":" + (IsWindowVisible(handle) ? "true" : "false") +
        ",\"keyboardLayout\":" + std::to_string(reinterpret_cast<uintptr_t>(layout)) +
        ",\"languageId\":" + std::to_string(LOWORD(reinterpret_cast<uintptr_t>(layout))));
}
struct Target { DWORD pid; HWND handle = nullptr; };
BOOL CALLBACK find_target(HWND handle, LPARAM data) {
    auto& target = *reinterpret_cast<Target*>(data); DWORD pid = 0; GetWindowThreadProcessId(handle, &pid);
    if (pid == target.pid && IsWindowVisible(handle) && !GetWindow(handle, GW_OWNER)) { target.handle = handle; return FALSE; }
    return TRUE;
}
bool tree(IUIAutomation* automation, HWND handle, size_t maximum) {
    ComPtr<IUIAutomationElement> root; checkpoint("candidate.ElementFromHandle");
    if (!emit("candidate.ElementFromHandle", automation->ElementFromHandle(handle, &root)) || !root) return false;
    ComPtr<IUIAutomationTreeWalker> walker;
    if (!emit("candidate.RawViewWalker", automation->get_RawViewWalker(&walker)) || !walker) return false;
    struct Entry { ComPtr<IUIAutomationElement> value; int depth; };
    std::vector<Entry> pending{{root, 0}}; size_t count = 0; bool depth_limited = false;
    while (!pending.empty() && count < maximum) {
        auto current = std::move(pending.back()); pending.pop_back();
        checkpoint("candidate.element"); element(current.value.Get(), "candidate.element", static_cast<int>(count++), current.depth);
        ComPtr<IUIAutomationElement> next;
        if (current.depth && SUCCEEDED(walker->GetNextSiblingElement(current.value.Get(), &next)) && next)
            pending.push_back({next, current.depth});
        ComPtr<IUIAutomationElement> child;
        if (SUCCEEDED(walker->GetFirstChildElement(current.value.Get(), &child)) && child) {
            if (current.depth < 8) pending.push_back({child, current.depth + 1});
            else depth_limited = true;
        }
    }
    emit("candidate.tree_complete", S_OK, ",\"nodes\":" + std::to_string(count) +
        ",\"truncated\":" + ((!pending.empty() || depth_limited) ? "true" : "false") + ",\"candidateIdentityAsserted\":false");
    return true;
}

int inspect(IUIAutomation* automation, DWORD pid, wchar_t const* id) {
    Target target{pid}; EnumWindows(find_target, reinterpret_cast<LPARAM>(&target));
    if (!emit("target_window", target.handle ? S_OK : HRESULT_FROM_WIN32(ERROR_NOT_FOUND))) return 2;
    window(target.handle, "fixture_window"); window(GetForegroundWindow(), "foreground_window");
    ComPtr<IUIAutomationElement> focused;
    checkpoint("GetFocusedElement"); auto hr = automation->GetFocusedElement(&focused);
    if (emit("GetFocusedElement", hr)) element(focused.Get(), "focused_element");
    ComPtr<IUIAutomationElement> root, editor;
    checkpoint("fixture.ElementFromHandle");
    if (!emit("fixture.ElementFromHandle", automation->ElementFromHandle(target.handle, &root))) return 2;
    VARIANT value; VariantInit(&value); value.vt = VT_BSTR; value.bstrVal = SysAllocString(id);
    if (!value.bstrVal) { emit("automationId.allocate", E_OUTOFMEMORY); return 2; }
    ComPtr<IUIAutomationCondition> condition;
    hr = automation->CreatePropertyCondition(UIA_AutomationIdPropertyId, value, &condition); VariantClear(&value);
    if (!emit("automationId.condition", hr)) return 2;
    checkpoint("editor.FindFirst"); hr = root->FindFirst(TreeScope_Descendants, condition.Get(), &editor);
    if (!emit("editor.FindFirst", FAILED(hr) ? hr : (editor ? S_OK : HRESULT_FROM_WIN32(ERROR_NOT_FOUND)))) return 2;
    int actual_pid = 0; hr = editor->get_CurrentProcessId(&actual_pid);
    if (!emit("editor.identity", FAILED(hr) ? hr : (actual_pid == static_cast<int>(pid) ? S_OK : E_UNEXPECTED))) return 2;
    element(editor.Get(), "editor");
    ComPtr<IUIAutomationValuePattern> value_pattern;
    hr = editor->GetCurrentPatternAs(UIA_ValuePatternId, IID_PPV_ARGS(&value_pattern));
    if (emit("Value.pattern", hr) && value_pattern) {
        Bstr content; hr = value_pattern->get_CurrentValue(&content.value);
        emit("Value.text", hr, SUCCEEDED(hr) ? ",\"text\":" + quoted(content.value) : "");
    }
    ComPtr<IUIAutomationTextPattern> text;
    hr = editor->GetCurrentPatternAs(UIA_TextPatternId, IID_PPV_ARGS(&text));
    if (emit("Text.pattern", hr) && text) {
        ComPtr<IUIAutomationTextRange> document; hr = text->get_DocumentRange(&document);
        if (emit("Text.document", hr)) {
            range(document.Get(), "document");
            terminal_range(document.Get());
        }
        ComPtr<IUIAutomationTextRangeArray> selection; hr = text->GetSelection(&selection);
        if (emit("Text.GetSelection", hr) && selection) {
            int count = 0; hr = selection->get_Length(&count);
            emit("selection.count", hr, ",\"count\":" + std::to_string(count) + ",\"limit\":16");
            if (SUCCEEDED(hr)) for (int i = 0; i < std::min(count, 16); ++i) {
                ComPtr<IUIAutomationTextRange> selected; hr = selection->GetElement(i, &selected);
                if (emit("selection.element", hr, ",\"index\":" + std::to_string(i))) range(selected.Get(), "selection." + std::to_string(i));
            }
        }
    }
    ComPtr<IUIAutomationTextEditPattern> edit;
    checkpoint("TextEdit.GetCurrentPatternAs"); hr = editor->GetCurrentPatternAs(UIA_TextEditPatternId, IID_PPV_ARGS(&edit));
    if (emit("TextEdit.pattern", hr) && edit) {
        ComPtr<IUIAutomationTextRange> active, conversion;
        checkpoint("TextEdit.GetActiveComposition"); hr = edit->GetActiveComposition(&active);
        if (emit("TextEdit.GetActiveComposition", hr)) range(active.Get(), "composition");
        checkpoint("TextEdit.GetConversionTarget"); hr = edit->GetConversionTarget(&conversion);
        if (emit("TextEdit.GetConversionTarget", hr)) range(conversion.Get(), "conversion");
    }
    ComPtr<IUIAutomationTextPattern2> text2;
    hr = editor->GetCurrentPatternAs(UIA_TextPattern2Id, IID_PPV_ARGS(&text2));
    if (emit("Text2.pattern", hr) && text2) {
        BOOL active = FALSE; ComPtr<IUIAutomationTextRange> caret;
        checkpoint("Text2.GetCaretRange"); hr = text2->GetCaretRange(&active, &caret);
        if (emit("Text2.GetCaretRange", hr, std::string(",\"active\":") + (active ? "true" : "false"))) range(caret.Get(), "caret");
    }
    Windows windows; EnumWindows(collect_window, reinterpret_cast<LPARAM>(&windows));
    for (auto handle : windows.values) window(handle, "visible_top_level_window");
    emit("visible_windows.complete", S_OK, ",\"count\":" + std::to_string(windows.values.size()) + ",\"truncated\":" + (windows.truncated ? "true" : "false"));
    Windows children; EnumChildWindows(target.handle, collect_window, reinterpret_cast<LPARAM>(&children));
    for (auto handle : children.values) window(handle, "visible_fixture_child_window");
    emit("fixture_child_windows.complete", S_OK, ",\"count\":" + std::to_string(children.values.size()) + ",\"truncated\":" + (children.truncated ? "true" : "false"));
    return 0;
}
} // namespace

int wmain(int argc, wchar_t** argv) {
    bool window_mode = argc >= 2 && std::wcscmp(argv[1], L"--window") == 0;
    if ((!window_mode && argc != 3) || (window_mode && (argc < 3 || argc > 4))) {
        std::cerr << "Usage: ime-uia.exe PID AutomationId\n       ime-uia.exe --window HWND [maxNodes=128]\nRead-only snapshots; supervise under an external deadline in the target interactive session.\n"; return 2;
    }
    wchar_t* end = nullptr; errno = 0;
    auto identifier = std::wcstoull(argv[window_mode ? 2 : 1], &end, 0);
    if (errno || *end || !identifier || identifier > (window_mode ? UINTPTR_MAX : MAXDWORD)) return 2;
    size_t maximum = 128;
    if (window_mode && argc == 4) {
        errno = 0; auto number = std::wcstoul(argv[3], &end, 10);
        if (errno || *end || !number || number > 512) return 2;
        maximum = static_cast<size_t>(number);
    }
    if (!window_mode && (!*argv[2] || std::wcslen(argv[2]) > 1024)) return 2;
    SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    auto hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (!emit("CoInitializeEx", hr)) return 2;
    int result = 2;
    {
        ComPtr<IUIAutomation> automation;
        hr = CoCreateInstance(__uuidof(CUIAutomation), nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&automation));
        if (emit("CoCreateInstance", hr)) {
            ComPtr<IUIAutomation2> options;
            if (SUCCEEDED(automation.As(&options))) {
                emit("ConnectionTimeout", options->put_ConnectionTimeout(3000));
                emit("TransactionTimeout", options->put_TransactionTimeout(3000));
            }
            if (window_mode) {
                auto handle = reinterpret_cast<HWND>(static_cast<uintptr_t>(identifier));
                if (IsWindow(handle)) { window(handle, "candidate_window"); result = tree(automation.Get(), handle, maximum) ? 0 : 2; }
                else emit("candidate_window", HRESULT_FROM_WIN32(ERROR_INVALID_WINDOW_HANDLE));
            } else result = inspect(automation.Get(), static_cast<DWORD>(identifier), argv[2]);
        }
        emit("complete", result ? E_FAIL : S_OK, ",\"readOnly\":true,\"semanticAssertions\":false");
    }
    CoUninitialize(); return result;
}

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <ole2.h>
#include <UIAutomation.h>
#include <wrl/client.h>
#include <iostream>
#include <iomanip>
#include <sstream>
#include <string>
#include <cstdlib>
#include <cerrno>
#include <cwchar>
using Microsoft::WRL::ComPtr;

// Read-only diagnostic. The caller must supervise this process with a deadline:
// UIA provider/native crashes cannot be made safe by catching HRESULT failures.
static std::string quoted(const wchar_t* text, UINT length) {
    std::ostringstream out; out << '"';
    for (UINT i = 0; i < length; ++i) {
        unsigned ch = static_cast<unsigned short>(text[i]);
        if (ch == '"' || ch == '\\') out << '\\' << static_cast<char>(ch);
        else if (ch >= 32 && ch < 127) out << static_cast<char>(ch);
        else out << "\\u" << std::hex << std::setfill('0') << std::setw(4) << ch << std::dec;
    }
    out << '"'; return out.str();
}
static std::string quoted(BSTR text) { return quoted(text, SysStringLen(text)); }
static bool emit(const char* stage, HRESULT hr, const std::string& details = "") {
    std::ostringstream code; code << "0x" << std::hex << std::setfill('0') << std::setw(8) << static_cast<unsigned long>(hr);
    std::cout << "{\"stage\":\"" << stage << "\",\"hresult\":\"" << code.str()
              << "\",\"succeeded\":" << (SUCCEEDED(hr) ? "true" : "false") << details << "}" << std::endl;
    return SUCCEEDED(hr);
}
static void checkpoint(const char* stage) {
    std::cout << "{\"checkpoint\":\"" << stage << "\"}" << std::endl;
}
struct Bstr { BSTR value = nullptr; ~Bstr() { SysFreeString(value); } };
struct WindowSearch { DWORD pid; HWND window = nullptr; };
static BOOL CALLBACK find_window(HWND window, LPARAM data) {
    auto& search = *reinterpret_cast<WindowSearch*>(data); DWORD pid = 0;
    GetWindowThreadProcessId(window, &pid);
    if (pid == search.pid && IsWindowVisible(window) && GetWindow(window, GW_OWNER) == nullptr) {
        search.window = window; return FALSE;
    }
    return TRUE;
}
static int inspect(DWORD pid, int max_length, const wchar_t* automation_id) {
    WindowSearch search{pid}; EnumWindows(find_window, reinterpret_cast<LPARAM>(&search));
    if (!emit("target_window", search.window ? S_OK : HRESULT_FROM_WIN32(ERROR_NOT_FOUND),
              ",\"pid\":" + std::to_string(pid) + ",\"hwnd\":" + std::to_string(reinterpret_cast<uintptr_t>(search.window)))) return 2;
    ComPtr<IUIAutomation> automation;
    if (!emit("CoCreateInstance_CUIAutomation", CoCreateInstance(__uuidof(CUIAutomation), nullptr,
        CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&automation)))) return 2;
    ComPtr<IUIAutomation2> automation2;
    HRESULT version = automation.As(&automation2); emit("QueryInterface_IUIAutomation2", version);
    if (SUCCEEDED(version)) {
        if (!emit("put_ConnectionTimeout_5000", automation2->put_ConnectionTimeout(5000))) return 2;
        if (!emit("put_TransactionTimeout_5000", automation2->put_TransactionTimeout(5000))) return 2;
    }
    ComPtr<IUIAutomationElement> root;
    checkpoint("ElementFromHandle");
    if (!emit("ElementFromHandle", automation->ElementFromHandle(search.window, &root))) return 2;
    VARIANT id; VariantInit(&id); id.vt = VT_BSTR; id.bstrVal = SysAllocString(automation_id);
    if (!id.bstrVal) { emit("allocate_AutomationId", E_OUTOFMEMORY); return 2; }
    ComPtr<IUIAutomationCondition> condition;
    HRESULT hr = automation->CreatePropertyCondition(UIA_AutomationIdPropertyId, id, &condition); VariantClear(&id);
    if (!emit("CreatePropertyCondition_native_editor", hr,
              ",\"automationId\":" + quoted(automation_id, static_cast<UINT>(std::wcslen(automation_id))))) return 2;
    ComPtr<IUIAutomationElement> editor;
    checkpoint("FindFirst_native_editor");
    hr = root->FindFirst(TreeScope_Descendants, condition.Get(), &editor);
    if (!emit("FindFirst_native_editor", FAILED(hr) ? hr : (editor ? S_OK : HRESULT_FROM_WIN32(ERROR_NOT_FOUND)))) return 2;
    int actual_pid = 0;
    hr = editor->get_CurrentProcessId(&actual_pid);
    if (!emit("editor_process_identity", FAILED(hr) ? hr : (actual_pid == static_cast<int>(pid) ? S_OK : E_UNEXPECTED),
              ",\"actualPid\":" + std::to_string(actual_pid))) return 2;
    ComPtr<IUIAutomationValuePattern> value;
    if (!emit("GetCurrentPatternAs_Value", editor->GetCurrentPatternAs(UIA_ValuePatternId, IID_PPV_ARGS(&value)))) return 2;
    Bstr content;
    checkpoint("ValuePattern_get_CurrentValue");
    hr = value->get_CurrentValue(&content.value);
    if (!emit("ValuePattern_get_CurrentValue", hr, SUCCEEDED(hr) ? ",\"text\":" + quoted(content.value) : "")) return 2;
    ComPtr<IUIAutomationTextPattern> text;
    if (!emit("GetCurrentPatternAs_Text", editor->GetCurrentPatternAs(UIA_TextPatternId, IID_PPV_ARGS(&text)))) return 2;
    ComPtr<IUIAutomationTextRange> document;
    checkpoint("TextPattern_get_DocumentRange");
    if (!emit("TextPattern_get_DocumentRange", text->get_DocumentRange(&document))) return 2;
    ComPtr<IUIAutomationTextRangeArray> ranges;
    checkpoint("TextPattern_GetSelection");
    if (!emit("TextPattern_GetSelection", text->GetSelection(&ranges))) return 2;
    if (!ranges) { emit("selection_ranges_present", E_POINTER); return 2; }
    int count = 0;
    hr = ranges->get_Length(&count);
    if (!emit("selection_range_count", hr, ",\"count\":" + std::to_string(count)) || count != 1) return 2;
    ComPtr<IUIAutomationTextRange> selected;
    if (!emit("selection_GetElement_0", ranges->GetElement(0, &selected))) return 2;
    auto compare = [&](const char* label, IUIAutomationTextRange* source, TextPatternRangeEndpoint first,
                       IUIAutomationTextRange* target, TextPatternRangeEndpoint second) {
        checkpoint(label); int result = 0;
        HRESULT comparison = source->CompareEndpoints(first, target, second, &result);
        return emit(label, comparison, ",\"ordering\":" + std::to_string(result));
    };
    if (!compare("document_start_to_end", document.Get(), TextPatternRangeEndpoint_Start, document.Get(), TextPatternRangeEndpoint_End)
        || !compare("selection_start_to_end", selected.Get(), TextPatternRangeEndpoint_Start, selected.Get(), TextPatternRangeEndpoint_End)
        || !compare("selection_start_to_document_start", selected.Get(), TextPatternRangeEndpoint_Start, document.Get(), TextPatternRangeEndpoint_Start)
        || !compare("selection_end_to_document_end", selected.Get(), TextPatternRangeEndpoint_End, document.Get(), TextPatternRangeEndpoint_End)) return 2;
    Bstr selection_text;
    checkpoint("selection_GetText");
    hr = selected->GetText(max_length, &selection_text.value);
    if (!emit("selection_GetText", hr, ",\"maxLength\":" + std::to_string(max_length)
        + (SUCCEEDED(hr) ? ",\"text\":" + quoted(selection_text.value) : ""))) return 2;
    Bstr document_text;
    checkpoint("document_GetText");
    hr = document->GetText(max_length, &document_text.value);
    if (!emit("document_GetText", hr, ",\"maxLength\":" + std::to_string(max_length)
        + (SUCCEEDED(hr) ? ",\"text\":" + quoted(document_text.value) : ""))) return 2;
    emit("complete", S_OK, ",\"readOnly\":true,\"semanticAssertions\":false"); return 0;
}
int wmain(int argc, wchar_t** argv) {
    if (argc < 2 || argc > 4) {
        std::cerr << "Usage: native-uia.exe PID [maxLength=-1] [AutomationId=native-editor]\nRead-only; run in the target interactive session under an external deadline.\n";
        return 2;
    }
    wchar_t* end = nullptr; errno = 0; unsigned long pid = std::wcstoul(argv[1], &end, 10);
    if (errno || *end || !pid) return 2;
    int max_length = -1;
    if (argc >= 3) {
        errno = 0; long parsed = std::wcstol(argv[2], &end, 10);
        if (errno || *end || (parsed != -1 && (parsed < 1 || parsed > 4096))) return 2;
        max_length = static_cast<int>(parsed);
    }
    const wchar_t* automation_id = argc == 4 ? argv[3] : L"native-editor";
    if (!*automation_id || std::wcslen(automation_id) > 1024) return 2;
    HRESULT hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (!emit("CoInitializeEx_MTA", hr)) return 2;
    int result = inspect(pid, max_length, automation_id); CoUninitialize(); return result;
}

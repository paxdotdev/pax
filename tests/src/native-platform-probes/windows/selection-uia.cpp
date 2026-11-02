#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <ole2.h>
#include <UIAutomation.h>
#include <wrl/client.h>
#include <cerrno>
#include <climits>
#include <cstdint>
#include <cstdio>
#include <cwchar>
#include <iomanip>
#include <sstream>
#include <string>
#include <vector>

using Microsoft::WRL::ComPtr;

// Fixture-only readback. The parent must enforce a process deadline: a provider
// crash or blocked COM call cannot be made safe by HRESULT handling in-process.
static FILE* output = nullptr;
static bool read_expected_fixture(const wchar_t* path, std::wstring& expected) {
    HANDLE file = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, nullptr,
        OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    LARGE_INTEGER size{};
    bool valid = GetFileSizeEx(file, &size) && size.QuadPart > 0 && size.QuadPart <= 65536;
    std::string bytes(valid ? static_cast<size_t>(size.QuadPart) : 0, '\0');
    DWORD read = 0;
    if (valid) valid = ReadFile(file, bytes.data(), static_cast<DWORD>(bytes.size()), &read, nullptr)
        && read == bytes.size();
    CloseHandle(file);
    if (!valid) return false;
    const int length = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS,
        bytes.data(), static_cast<int>(bytes.size()), nullptr, 0);
    if (!length) return false;
    expected.resize(length);
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, bytes.data(),
        static_cast<int>(bytes.size()), expected.data(), length) == length;
}
static std::string quoted(const wchar_t* value, size_t length) {
    std::ostringstream out; out << '"';
    for (size_t i = 0; i < length; ++i) {
        const unsigned ch = static_cast<unsigned short>(value[i]);
        if (ch == '"' || ch == '\\') out << '\\' << static_cast<char>(ch);
        else if (ch >= 32 && ch < 127) out << static_cast<char>(ch);
        else out << "\\u" << std::hex << std::setfill('0') << std::setw(4) << ch << std::dec;
    }
    out << '"'; return out.str();
}
static void line(const std::string& value) {
    std::fprintf(output, "%s\n", value.c_str());
    std::fflush(output); // Preserve the last checkpoint even if the next call AVs.
}
static void checkpoint(const char* stage, int index = -1) {
    line("{\"checkpoint\":\"" + std::string(stage) + "\",\"range\":" + std::to_string(index) + "}");
}
static bool emit(const char* stage, HRESULT hr, const std::string& details = "") {
    std::ostringstream code;
    code << "0x" << std::hex << std::setfill('0') << std::setw(8) << static_cast<uint32_t>(hr);
    line("{\"stage\":\"" + std::string(stage) + "\",\"hresult\":\"" + code.str()
        + "\",\"succeeded\":" + (SUCCEEDED(hr) ? "true" : "false") + details + "}");
    return SUCCEEDED(hr);
}
struct Bstr { BSTR value = nullptr; ~Bstr() { SysFreeString(value); } };
struct WindowSearch { DWORD pid; std::vector<HWND> handles; };
static BOOL CALLBACK find_windows(HWND hwnd, LPARAM parameter) {
    auto& windows = *reinterpret_cast<WindowSearch*>(parameter);
    DWORD pid = 0; GetWindowThreadProcessId(hwnd, &pid);
    if (pid == windows.pid && IsWindowVisible(hwnd)) windows.handles.push_back(hwnd);
    return windows.handles.size() < 64;
}

static int inspect(DWORD pid, const wchar_t* automation_id, int max_chars, const std::wstring& expected_text) {
    emit("request", S_OK, ",\"pid\":" + std::to_string(pid) + ",\"automationId\":"
        + quoted(automation_id, std::wcslen(automation_id)) + ",\"maxChars\":" + std::to_string(max_chars));
    ComPtr<IUIAutomation> automation;
    checkpoint("CoCreateInstance");
    if (!emit("CoCreateInstance", CoCreateInstance(__uuidof(CUIAutomation), nullptr,
        CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&automation)))) return 2;
    ComPtr<IUIAutomation2> automation2;
    const HRESULT optional = automation.As(&automation2);
    emit("optional_IUIAutomation2", optional);
    if (FAILED(optional) && optional != E_NOINTERFACE) return 2;
    if (automation2) {
        if (!emit("ConnectionTimeout", automation2->put_ConnectionTimeout(5000))
            || !emit("TransactionTimeout", automation2->put_TransactionTimeout(5000))) return 2;
    }
    VARIANT id; VariantInit(&id); id.vt = VT_BSTR; id.bstrVal = SysAllocString(automation_id);
    if (!id.bstrVal) { emit("AutomationId", E_OUTOFMEMORY); return 2; }
    ComPtr<IUIAutomationCondition> id_condition;
    const HRESULT id_hr = automation->CreatePropertyCondition(UIA_AutomationIdPropertyId, id, &id_condition);
    VariantClear(&id);
    if (!emit("AutomationIdCondition", id_hr)) return 2;
    VARIANT process; VariantInit(&process); process.vt = VT_I4; process.lVal = static_cast<LONG>(pid);
    ComPtr<IUIAutomationCondition> process_condition, condition;
    if (!emit("ProcessCondition", automation->CreatePropertyCondition(UIA_ProcessIdPropertyId, process, &process_condition))
        || !emit("AndCondition", automation->CreateAndCondition(id_condition.Get(), process_condition.Get(), &condition))) return 2;

    WindowSearch windows{pid, {}}; EnumWindows(find_windows, reinterpret_cast<LPARAM>(&windows));
    if (!emit("TargetWindows", windows.handles.empty() ? HRESULT_FROM_WIN32(ERROR_NOT_FOUND) : S_OK,
        ",\"count\":" + std::to_string(windows.handles.size()))) return 2;
    ComPtr<IUIAutomationElement> element;
    for (HWND hwnd : windows.handles) {
        ComPtr<IUIAutomationElement> root;
        checkpoint("ElementFromHandle");
        if (!emit("ElementFromHandle", automation->ElementFromHandle(hwnd, &root))) return 2;
        if (!root) { emit("WindowElement", E_POINTER); return 2; }
        checkpoint("FindFirstExactTarget");
        if (!emit("FindFirstExactTarget", root->FindFirst(TreeScope_Subtree, condition.Get(), &element))) return 2;
        if (element) break;
    }
    if (!element) { emit("ExactTarget", HRESULT_FROM_WIN32(ERROR_NOT_FOUND)); return 2; }
    int actual_pid = 0; Bstr actual_id;
    checkpoint("TargetIdentity");
    if (!emit("TargetProcessId", element->get_CurrentProcessId(&actual_pid))
        || !emit("TargetAutomationId", element->get_CurrentAutomationId(&actual_id.value))) return 2;
    const bool identity_matches = actual_pid == static_cast<int>(pid) && actual_id.value
        && SysStringLen(actual_id.value) == std::wcslen(automation_id)
        && std::wmemcmp(actual_id.value, automation_id, SysStringLen(actual_id.value)) == 0;
    if (!emit("ExactTarget", identity_matches ? S_OK : E_UNEXPECTED,
        ",\"actualPid\":" + std::to_string(actual_pid) + ",\"identityMatches\":" + (identity_matches ? "true" : "false"))) return 2;
    ComPtr<IUIAutomationTextPattern> text;
    checkpoint("TextPattern");
    if (!emit("TextPattern", element->GetCurrentPatternAs(UIA_TextPatternId, IID_PPV_ARGS(&text)))) return 2;
    if (!text) { emit("TextPatternPresent", E_POINTER); return 2; }
    ComPtr<IUIAutomationTextRangeArray> ranges;
    checkpoint("GetSelection");
    if (!emit("GetSelection", text->GetSelection(&ranges))) return 2;
    if (!ranges) { emit("SelectionPresent", E_POINTER); return 2; }
    int count = 0;
    checkpoint("SelectionCount");
    if (!emit("SelectionCount", ranges->get_Length(&count), "")) return 2;
    if (!emit("SelectionCountBound", count >= 0 && count <= 64 ? S_OK : E_UNEXPECTED,
        ",\"count\":" + std::to_string(count))) return 2;
    bool matches = count == 1;
    for (int index = 0; index < count; ++index) {
        ComPtr<IUIAutomationTextRange> range;
        checkpoint("SelectionElement", index);
        if (!emit("SelectionElement", ranges->GetElement(index, &range), ",\"range\":" + std::to_string(index))) return 2;
        if (!range) { emit("SelectionElementPresent", E_POINTER); return 2; }
        Bstr selected;
        checkpoint("GetText", index);
        const HRESULT hr = range->GetText(max_chars, &selected.value);
        const UINT length = SUCCEEDED(hr) ? SysStringLen(selected.value) : 0;
        const bool exact = SUCCEEDED(hr) && length == expected_text.size()
            && std::wmemcmp(selected.value, expected_text.data(), length) == 0;
        if (!emit("GetText", hr, ",\"range\":" + std::to_string(index) + ",\"length\":" + std::to_string(length)
            + ",\"matchesExpected\":" + (exact ? "true" : "false")
            + (exact && expected_text == L"Home link" ? ",\"text\":\"Home link\"" : ""))) return 2;
        matches = matches && exact;
    }
    emit("complete", S_OK, ",\"pid\":" + std::to_string(pid) + ",\"selectionCount\":" + std::to_string(count)
        + ",\"matchesExpected\":" + (matches ? "true" : "false") + ",\"readOnly\":true");
    return matches ? 0 : 3;
}

int wmain(int argc, wchar_t** argv) {
    if (argc != 5 && argc != 6) {
        std::fprintf(stderr, "Usage: selection-uia.exe PID AutomationId maxChars output.jsonl [expected-fixture-utf8.txt]\n"
            "maxChars: 1..4096, or explicit -1 reproduction. Only expected fixture text Home link is printed.\n"
            "Optional expected fixture: exact UTF-8 without BOM, 1..65536 bytes; only match/length are reported.\n"
            "Run in the target interactive session under an external 20-second deadline.\n");
        return 2;
    }
    wchar_t* end = nullptr; errno = 0;
    const unsigned long pid = std::wcstoul(argv[1], &end, 10);
    if (errno || end == argv[1] || *end || !pid || pid > INT_MAX) return 2;
    if (!*argv[2] || std::wcslen(argv[2]) > 1024) return 2;
    errno = 0; const long max_chars = std::wcstol(argv[3], &end, 10);
    if (errno || end == argv[3] || *end || (max_chars != -1 && (max_chars < 1 || max_chars > 4096))) return 2;
    std::wstring expected_text = L"Home link";
    if (argc == 6 && !read_expected_fixture(argv[5], expected_text)) return 2;
    if (_wfopen_s(&output, argv[4], L"wb") != 0 || !output) return 2;
    checkpoint("CoInitializeEx_MTA");
    const HRESULT initialized = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    int result = 2;
    if (emit("CoInitializeEx_MTA", initialized)) {
        result = inspect(static_cast<DWORD>(pid), argv[2], static_cast<int>(max_chars), expected_text);
        CoUninitialize();
    }
    std::fclose(output); return result;
}

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
#include <limits>
#include <sstream>
#include <string>

using Microsoft::WRL::ComPtr;

// Read-only fixture observations. Run in a separate MTA process: managed UIA
// can block on this Server image, and even native calls need an external deadline.
static FILE* output = nullptr;
static void line(const std::string& value) {
    std::fprintf(output, "%s\n", value.c_str());
    std::fflush(output);
}
static void checkpoint(const char* stage, int index = -1) {
    line("{\"checkpoint\":\"" + std::string(stage) + "\",\"index\":" + std::to_string(index) + "}");
}
static bool succeeded(const char* stage, HRESULT hr) {
    if (SUCCEEDED(hr)) return true;
    std::ostringstream value;
    value << "{\"failed\":\"" << stage << "\",\"hresult\":\"0x" << std::hex
          << std::setw(8) << std::setfill('0') << static_cast<uint32_t>(hr) << "\"}";
    line(value.str()); return false;
}
struct Bstr { BSTR value = nullptr; ~Bstr() { SysFreeString(value); } };
static std::string quoted(BSTR value) {
    std::ostringstream out; out << '"';
    for (UINT i = 0; i < SysStringLen(value); ++i) {
        const unsigned ch = static_cast<unsigned short>(value[i]);
        if (ch == '"' || ch == '\\') out << '\\' << static_cast<char>(ch);
        else if (ch >= 32 && ch < 127) out << static_cast<char>(ch);
        else out << "\\u" << std::hex << std::setw(4) << std::setfill('0') << ch << std::dec;
    }
    out << '"'; return out.str();
}

static int inspect(DWORD pid, HWND hwnd) {
    DWORD actual_pid = 0; GetWindowThreadProcessId(hwnd, &actual_pid);
    if (!IsWindow(hwnd) || !IsWindowVisible(hwnd) || actual_pid != pid) {
        line("{\"failed\":\"Window ownership\"}"); return 2;
    }
    ComPtr<IUIAutomation> automation;
    checkpoint("CoCreateInstance");
    if (!succeeded("CoCreateInstance", CoCreateInstance(__uuidof(CUIAutomation), nullptr,
        CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&automation)))) return 2;
    ComPtr<IUIAutomation2> automation2;
    const HRESULT optional = automation.As(&automation2);
    if (FAILED(optional) && optional != E_NOINTERFACE) {
        succeeded("IUIAutomation2", optional); return 2;
    }
    if (automation2 && (!succeeded("ConnectionTimeout", automation2->put_ConnectionTimeout(5000))
        || !succeeded("TransactionTimeout", automation2->put_TransactionTimeout(5000)))) return 2;

    VARIANT process; VariantInit(&process); process.vt = VT_I4; process.lVal = static_cast<LONG>(pid);
    VARIANT scroll; VariantInit(&scroll); scroll.vt = VT_BOOL; scroll.boolVal = VARIANT_TRUE;
    VARIANT label; VariantInit(&label); label.vt = VT_BSTR;
    label.bstrVal = SysAllocString(L"ROOT OVERLAY \x00b7 No viewport owns this input");
    if (!label.bstrVal) return 2;
    ComPtr<IUIAutomationCondition> process_condition, scroll_condition, label_condition, wanted, condition;
    const HRESULT label_hr = automation->CreatePropertyCondition(UIA_NamePropertyId, label, &label_condition);
    VariantClear(&label);
    if (!succeeded("Label condition", label_hr)
        || !succeeded("Process condition", automation->CreatePropertyCondition(UIA_ProcessIdPropertyId, process, &process_condition))
        || !succeeded("Scroll condition", automation->CreatePropertyCondition(UIA_IsScrollPatternAvailablePropertyId, scroll, &scroll_condition))
        || !succeeded("Wanted condition", automation->CreateOrCondition(scroll_condition.Get(), label_condition.Get(), &wanted))
        || !succeeded("Owned condition", automation->CreateAndCondition(process_condition.Get(), wanted.Get(), &condition))) return 2;
    ComPtr<IUIAutomationElement> root;
    checkpoint("ElementFromHandle");
    if (!succeeded("ElementFromHandle", automation->ElementFromHandle(hwnd, &root)) || !root) return 2;
    ComPtr<IUIAutomationElementArray> elements;
    checkpoint("FindAllOwnedScrollAndOverlay");
    if (!succeeded("FindAll", root->FindAll(TreeScope_Subtree, condition.Get(), &elements)) || !elements) return 2;
    int count = 0;
    if (!succeeded("Count", elements->get_Length(&count)) || count < 0 || count > 64) return 2;
    std::ostringstream rows;
    rows << std::setprecision(17) << '[';
    for (int index = 0; index < count; ++index) {
        checkpoint("Element", index);
        ComPtr<IUIAutomationElement> element;
        if (!succeeded("GetElement", elements->GetElement(index, &element)) || !element) return 2;
        int element_pid = 0;
        Bstr id, name, class_name;
        RECT bounds{};
        if (!succeeded("ProcessId", element->get_CurrentProcessId(&element_pid)) || element_pid != static_cast<int>(pid)
            || !succeeded("AutomationId", element->get_CurrentAutomationId(&id.value))
            || !succeeded("Name", element->get_CurrentName(&name.value))
            || !succeeded("ClassName", element->get_CurrentClassName(&class_name.value))
            || !succeeded("Bounds", element->get_CurrentBoundingRectangle(&bounds))) return 2;
        if (index) rows << ',';
        rows << "{\"id\":" << quoted(id.value) << ",\"name\":" << quoted(name.value)
             << ",\"class\":" << quoted(class_name.value) << ",\"pid\":" << element_pid
             << ",\"left\":" << bounds.left << ",\"top\":" << bounds.top
             << ",\"width\":" << bounds.right - bounds.left << ",\"height\":" << bounds.bottom - bounds.top;
        VARIANT available; VariantInit(&available);
        checkpoint("ScrollPatternAvailable", index);
        const HRESULT available_hr = element->GetCurrentPropertyValue(UIA_IsScrollPatternAvailablePropertyId, &available);
        const bool is_scroll = SUCCEEDED(available_hr) && available.vt == VT_BOOL && available.boolVal == VARIANT_TRUE;
        VariantClear(&available);
        if (!succeeded("ScrollPatternAvailable", available_hr)) return 2;
        rows << ",\"scroll\":" << (is_scroll ? "true" : "false");
        if (is_scroll) {
            ComPtr<IUIAutomationScrollPattern> pattern;
            checkpoint("ScrollPattern", index);
            if (!succeeded("ScrollPattern", element->GetCurrentPatternAs(UIA_ScrollPatternId, IID_PPV_ARGS(&pattern))) || !pattern) return 2;
            double percent_x = 0, percent_y = 0, view_x = 0, view_y = 0;
            checkpoint("ScrollProperties", index);
            if (!succeeded("HorizontalScrollPercent", pattern->get_CurrentHorizontalScrollPercent(&percent_x))
                || !succeeded("VerticalScrollPercent", pattern->get_CurrentVerticalScrollPercent(&percent_y))
                || !succeeded("HorizontalViewSize", pattern->get_CurrentHorizontalViewSize(&view_x))
                || !succeeded("VerticalViewSize", pattern->get_CurrentVerticalViewSize(&view_y))) return 2;
            rows << ",\"percentX\":" << percent_x << ",\"percentY\":" << percent_y
                 << ",\"viewX\":" << view_x << ",\"viewY\":" << view_y;
        }
        rows << '}';
    }
    rows << ']';
    line("{\"stage\":\"complete\",\"readOnly\":true,\"pid\":" + std::to_string(pid) + ",\"elements\":" + rows.str() + "}");
    return 0;
}

int wmain(int argc, wchar_t** argv) {
    if (argc != 4) {
        std::fprintf(stderr, "Usage: wheel-uia.exe PID HWND-decimal output.jsonl\n"
            "Read-only ScrollPattern and exact fixture-overlay observations. Enforce an external process deadline.\n");
        return 2;
    }
    wchar_t* end = nullptr; errno = 0;
    const unsigned long pid = std::wcstoul(argv[1], &end, 10);
    if (errno || end == argv[1] || *end || !pid || pid > INT_MAX) return 2;
    errno = 0;
    const unsigned long long handle = std::wcstoull(argv[2], &end, 10);
    if (errno || end == argv[2] || *end || !handle || handle > (std::numeric_limits<uintptr_t>::max)()) return 2;
    if (_wfopen_s(&output, argv[3], L"wb") || !output) return 2;
    checkpoint("CoInitializeEx_MTA");
    const HRESULT initialized = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    int result = 2;
    if (succeeded("CoInitializeEx", initialized)) {
        result = inspect(static_cast<DWORD>(pid), reinterpret_cast<HWND>(static_cast<uintptr_t>(handle)));
        CoUninitialize();
    }
    std::fclose(output); return result;
}

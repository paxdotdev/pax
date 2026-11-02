#include "../../../../pax-host/native/windows/keyboard.h"
#include <iostream>
#include <stdexcept>

using pax::windows::keyboard_key;

void require(bool value, char const* message) {
    if (!value) throw std::runtime_error(message);
}

int main() {
    BYTE original_state[256]{};
    GetKeyboardState(original_state);
    HKL original_layout = GetKeyboardLayout(0);
    auto restore = [&] {
        SetKeyboardState(original_state);
        ActivateKeyboardLayout(original_layout, 0);
    };
    try {
        // This changes only the diagnostic thread's keyboard state; no keys
        // are injected into the desktop or a native editor.
        HKL layout = LoadKeyboardLayoutW(L"00000409", KLF_NOTELLSHELL);
        require(layout != nullptr, "US layout unavailable");
        ActivateKeyboardLayout(layout, 0);
        BYTE state[256]{};
        require(SetKeyboardState(state), "SetKeyboardState failed");
        auto scan = MapVirtualKeyExW('A', MAPVK_VK_TO_VSC, layout);
        require(keyboard_key('A', scan).name == L"a", "plain key does not follow layout");
        state[VK_SHIFT] = state[VK_LSHIFT] = 0x80;
        SetKeyboardState(state);
        require(keyboard_key('A', scan).name == L"A", "Shift lost");
        state[VK_SHIFT] = state[VK_LSHIFT] = 0;
        state[VK_CONTROL] = state[VK_LCONTROL] = 0x80;
        SetKeyboardState(state);
        require(keyboard_key('A', scan).name == L"a", "Control shortcut became a control character");
        require(keyboard_key(VK_LEFT, 0).name == L"ArrowLeft", "arrow name mismatch");
        require(keyboard_key(VK_F12, 0).name == L"F12", "function-key name mismatch");
        require(keyboard_key(VK_RETURN, 0).name == L"Enter", "Enter name mismatch");
        require(!keyboard_key(VK_RETURN, 0).printable, "Enter marked as printable text");

        layout = LoadKeyboardLayoutW(L"00020409", KLF_NOTELLSHELL);
        require(layout != nullptr, "US International layout unavailable");
        ActivateKeyboardLayout(layout, 0);
        ZeroMemory(state, sizeof(state)); SetKeyboardState(state);
        wchar_t buffer[16]{};
        auto accent_scan = MapVirtualKeyExW(VK_OEM_7, MAPVK_VK_TO_VSC, layout);
        auto e_scan = MapVirtualKeyExW('E', MAPVK_VK_TO_VSC, layout);
        require(keyboard_key(VK_OEM_7, accent_scan).name == L"Dead", "dead-key name mismatch");
        require(keyboard_key('E', e_scan).name == L"e", "naming a dead key changed native composition state");
        require(ToUnicodeEx(VK_OEM_7, accent_scan, state, buffer, 16, 0, layout) < 0,
            "diagnostic could not prime dead key");
        require(keyboard_key('E', e_scan).name == L"\u00e9", "pending dead-key composition ignored");
        require(keyboard_key('E', e_scan).name == L"\u00e9", "key lookup consumed native dead-key composition");
        require(ToUnicodeEx('E', e_scan, state, buffer, 16, 0, layout) == 1 && buffer[0] == L'\u00e9',
            "native translation lost pending composition");
        restore();
        std::cout << "PASS keyboard layout, modifiers, named keys, and dead-key state preservation\n";
        return 0;
    } catch (std::exception const& error) {
        restore();
        std::cerr << error.what() << '\n';
        return 1;
    }
}

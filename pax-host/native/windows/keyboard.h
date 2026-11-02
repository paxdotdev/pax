#pragma once
#include <windows.h>
#include <iterator>
#include <string>

namespace pax::windows {
struct KeyboardKey {
    std::wstring name;
    bool printable = false;
};

inline KeyboardKey keyboard_key(UINT key, UINT scan_code) {
    switch (key) {
    case VK_RETURN: return {L"Enter"};
    case VK_BACK: return {L"Backspace"};
    case VK_TAB: return {L"Tab"};
    case VK_ESCAPE: return {L"Escape"};
    case VK_DELETE: return {L"Delete"};
    case VK_INSERT: return {L"Insert"};
    case VK_HOME: return {L"Home"};
    case VK_END: return {L"End"};
    case VK_PRIOR: return {L"PageUp"};
    case VK_NEXT: return {L"PageDown"};
    case VK_LEFT: return {L"ArrowLeft"};
    case VK_RIGHT: return {L"ArrowRight"};
    case VK_UP: return {L"ArrowUp"};
    case VK_DOWN: return {L"ArrowDown"};
    case VK_SHIFT: case VK_LSHIFT: case VK_RSHIFT: return {L"Shift"};
    case VK_CONTROL: case VK_LCONTROL: case VK_RCONTROL: return {L"Control"};
    case VK_MENU: case VK_LMENU: case VK_RMENU: return {L"Alt"};
    case VK_LWIN: case VK_RWIN: return {L"Meta"};
    case VK_CAPITAL: return {L"CapsLock"};
    case VK_NUMLOCK: return {L"NumLock"};
    case VK_SCROLL: return {L"ScrollLock"};
    case VK_PAUSE: return {L"Pause"};
    case VK_SNAPSHOT: return {L"PrintScreen"};
    case VK_APPS: return {L"ContextMenu"};
    case VK_PROCESSKEY: return {L"Process"};
    default: break;
    }
    if (key >= VK_F1 && key <= VK_F24) return {L"F" + std::to_wstring(key - VK_F1 + 1)};
    BYTE state[256]{};
    if (!GetKeyboardState(state)) return {L"Unidentified"};
    // Ctrl shortcuts still name the printable key. Preserve Ctrl+Alt, which
    // layouts can use for AltGr characters, and preserve Shift/Caps Lock.
    if (!(state[VK_MENU] & 0x80)) state[VK_CONTROL] = state[VK_LCONTROL] = state[VK_RCONTROL] = 0;
    wchar_t buffer[16]{};
    // Flag 4 keeps TranslateMessage/the native editor's dead-key state intact.
    // This bridge names canvas keys; it never inserts text into a native editor.
    int count = ToUnicodeEx(key, scan_code, state, buffer, int(std::size(buffer)), 4, GetKeyboardLayout(0));
    if (count < 0) return {L"Dead"};
    if (count > 0 && buffer[0] >= 0x20 && buffer[0] != 0x7f)
        return {std::wstring(buffer, size_t(count)), true};
    return {L"Unidentified"};
}
} // namespace pax::windows

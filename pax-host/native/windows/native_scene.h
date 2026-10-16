#pragma once
#include "json_helpers.h"
#include "native_mask.h"
#include <functional>
#include <map>
#include <memory>
#include <optional>
#include <set>
#include <winrt/Windows.UI.Text.h>
#include <winrt/Microsoft.UI.h>
#include <winrt/Microsoft.UI.Input.h>
#include <winrt/Microsoft.UI.Xaml.Automation.h>
#include <winrt/Microsoft.UI.Xaml.Controls.Primitives.h>
#include <winrt/Microsoft.UI.Xaml.Hosting.h>
#include <winrt/Microsoft.UI.Xaml.Input.h>

namespace pax::windows {
namespace xaml = winrt::Microsoft::UI::Xaml;
namespace controls = xaml::Controls;
namespace media = xaml::Media;
namespace composition = winrt::Microsoft::UI::Composition;

class NativeScene {
public:
    using Interrupt = std::function<bool(json::JsonObject const&)>;
private:
    struct EventHandlers {
        std::shared_ptr<bool> live = std::make_shared<bool>(true);
        std::vector<std::function<void()>> revoke;
        void retire() noexcept {
            // Detached controls may still be retained by UI Automation. The
            // guard also protects against an already-queued native callback.
            *live = false;
            for (auto& remove : revoke) { try { remove(); } catch (...) {} }
            revoke.clear();
        }
        void clear() { retire(); live = std::make_shared<bool>(true); }
        ~EventHandlers() { retire(); }
    };
    struct Node {
        std::wstring kind;
        uint32_t id = 0, layer = 0;
        std::optional<uint32_t> parent;
        controls::Grid host{nullptr};
        xaml::FrameworkElement control{nullptr};
        controls::Canvas children{nullptr};
        controls::Canvas attached_to{nullptr};
        controls::Canvas mask_input{nullptr};
        media::Matrix world{1, 0, 0, 1, 0, 0};
        double width = 0, height = 0;
        composition::CompositionVisualSurface capture{nullptr};
        composition::SpriteVisual masked{nullptr};
        composition::CompositionDrawingSurface mask{nullptr};
        std::vector<MaskHole> holes;
        std::optional<MaskPath> clip_path;
        bool clip_content = false;
        bool mask_dirty = false;
        bool focus_pending = false;
        std::optional<uint64_t> measure_generation;
        EventHandlers events, mask_events;
    };
    controls::Canvas root_;
    controls::Button focus_parking_;
    std::map<uint32_t, std::unique_ptr<Node>> nodes_;
    struct Layer {
        controls::Canvas canvas;
        controls::Canvas parent{nullptr};
    };
    std::map<uint32_t, Layer> layers_;
    NativeMaskPainter masks_;
    composition::Compositor compositor_{nullptr};
    Interrupt interrupt_;
    std::function<winrt::hstring(json::JsonObject const&)> local_font_;
    std::function<void(winrt::hstring const&)> load_image_;
    bool applying_ = false;
    float dpr_ = 1;
    uint32_t layer_count_ = 1;
    EventHandlers root_events_;

    static media::SolidColorBrush color(json::IJsonValue const& value) {
        auto object = value.GetObject();
        auto rgba = object.HasKey(L"Rgba") ? object.GetNamedArray(L"Rgba") : object.GetNamedArray(L"Rgb");
        auto byte = [&](uint32_t index) { return uint8_t(std::round(std::clamp(rgba.GetNumberAt(index), 0.0, 1.0) * 255)); };
        return media::SolidColorBrush(winrt::Windows::UI::Color{rgba.Size() == 4 ? byte(3) : uint8_t(255), byte(0), byte(1), byte(2)});
    }
    static media::Matrix relative(media::Matrix const& child, media::Matrix const& parent) {
        double det = parent.M11 * parent.M22 - parent.M12 * parent.M21;
        if (!std::isfinite(det) || std::abs(det) < 1e-12) throw std::invalid_argument("Singular native parent transform");
        double a = parent.M22 / det, b = -parent.M12 / det, c = -parent.M21 / det, d = parent.M11 / det;
        double x = -(parent.OffsetX * a + parent.OffsetY * c), y = -(parent.OffsetX * b + parent.OffsetY * d);
        return {child.M11 * a + child.M12 * c, child.M11 * b + child.M12 * d,
            child.M21 * a + child.M22 * c, child.M21 * b + child.M22 * d,
            child.OffsetX * a + child.OffsetY * c + x, child.OffsetX * b + child.OffsetY * d + y};
    }
    void send(wchar_t const* kind, uint32_t id, wchar_t const* field = nullptr, json::IJsonValue const& value = nullptr) {
        if (applying_) return;
        json::JsonObject args; args.Insert(L"id", numeric(id));
        if (field) args.Insert(field, value);
        interrupt_(event(kind, args));
    }
    void pointer(xaml::Input::PointerRoutedEventArgs const& args, wchar_t const* kind) {
        auto point = args.GetCurrentPoint(root_);
        auto p = point.Position();
        json::JsonObject value; value.Insert(L"x", numeric(p.X)); value.Insert(L"y", numeric(p.Y));
        auto props = point.Properties();
        value.Insert(L"button", text(props.IsRightButtonPressed() ? L"Right" : props.IsMiddleButtonPressed() ? L"Middle" : L"Left"));
        value.Insert(L"modifiers", json::JsonArray());
        args.Handled(interrupt_(event(kind, value)));
    }
    void wire_pointer(xaml::UIElement const& element, EventHandlers& handlers, bool removed_region = false) {
        auto live = handlers.live;
        auto pressed = element.PointerPressed([this, live, removed_region](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
            if (!*live) return;
            pointer(args, L"MouseDown"); if (removed_region) args.Handled(true);
        });
        handlers.revoke.push_back([element, pressed] { element.PointerPressed(pressed); });
        auto released = element.PointerReleased([this, live, removed_region](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
            if (!*live) return;
            pointer(args, L"MouseUp"); pointer(args, L"Click"); if (removed_region) args.Handled(true);
        });
        handlers.revoke.push_back([element, released] { element.PointerReleased(released); });
        auto moved = element.PointerMoved([this, live, removed_region](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
            if (!*live) return;
            pointer(args, L"MouseMove"); if (removed_region) args.Handled(true);
        });
        handlers.revoke.push_back([element, moved] { element.PointerMoved(moved); });
    }
    Node& require(uint32_t id) {
        auto found = nodes_.find(id);
        if (found == nodes_.end()) throw std::invalid_argument("Native patch refers to an absent node");
        return *found->second;
    }
    static void detach(Node& node) {
        if (!node.attached_to) return;
        uint32_t index = 0;
        if (node.attached_to.Children().IndexOf(node.host, index)) node.attached_to.Children().RemoveAt(index);
        node.attached_to = nullptr;
    }
    void placement(Node& node) {
        auto target = root_;
        media::Matrix parent{1, 0, 0, 1, 0, 0};
        if (node.parent) {
            auto& owner = require(*node.parent);
            if (!owner.children) throw std::invalid_argument("Native parent is not a Frame or Scroller");
            target = owner.children; parent = owner.world;
        }
        if (node.attached_to != target) { detach(node); target.Children().Append(node.host); node.attached_to = target; }
        media::MatrixTransform transform;
        transform.Matrix(relative(node.world, parent)); node.host.RenderTransform(transform);
        node.host.Width(node.width); node.host.Height(node.height);
        node.control.Width(node.width); node.control.Height(node.height);
        if (node.clip_content && !node.clip_path) {
            media::RectangleGeometry clip;
            clip.Rect({0, 0, float(node.width), float(node.height)});
            node.host.Clip(clip);
        } else node.host.Clip(nullptr);
    }
    template<typename Control> void style(Control const& control, json::JsonObject const& style) {
        if (present(style, L"font_size")) control.FontSize(number(style, L"font_size"));
        if (present(style, L"fill")) control.Foreground(color(style.GetNamedValue(L"fill")));
        if (present(style, L"font")) {
            auto font = style.GetNamedObject(L"font");
            auto system = font.GetNamedObject(font.HasKey(L"Local") ? L"Local" : font.HasKey(L"Web") ? L"Web" : L"System");
            if (font.HasKey(L"Local")) control.FontFamily(media::FontFamily(local_font_(system)));
            else if (font.HasKey(L"Web") && !string(system, L"url").empty()) {
                auto url = system.GetNamedString(L"url");
                if (std::wstring_view(url).find(L"://") != std::wstring_view::npos)
                    throw std::invalid_argument("Remote font URLs/CSS require an explicit native font download policy");
                json::JsonObject local;
                local.Insert(L"path", text(url)); local.Insert(L"family", text(string(system, L"family")));
                control.FontFamily(media::FontFamily(local_font_(local)));
            }
            else if (present(system, L"family")) control.FontFamily(media::FontFamily(system.GetNamedString(L"family")));
            if (present(system, L"weight")) {
                static const std::map<std::wstring, uint16_t> weights{{L"Thin",uint16_t{100}},{L"ExtraLight",uint16_t{200}},{L"Light",uint16_t{300}},{L"Normal",uint16_t{400}},{L"Medium",uint16_t{500}},{L"SemiBold",uint16_t{600}},{L"Bold",uint16_t{700}},{L"ExtraBold",uint16_t{800}},{L"Black",uint16_t{900}}};
                control.FontWeight(winrt::Windows::UI::Text::FontWeight{weights.at(system.GetNamedString(L"weight").c_str())});
            }
            if (present(system, L"style")) {
                auto s = system.GetNamedString(L"style");
                control.FontStyle(s == L"Italic" ? winrt::Windows::UI::Text::FontStyle::Italic : s == L"Oblique" ? winrt::Windows::UI::Text::FontStyle::Oblique : winrt::Windows::UI::Text::FontStyle::Normal);
            }
        }
    }
    void create(std::wstring const& kind, json::JsonObject const& patch) {
        uint32_t id = uint32_t(number(patch, L"id"));
        if (nodes_.contains(id)) throw std::invalid_argument("Duplicate native creation");
        auto node = std::make_unique<Node>(); node->id = id; node->kind = kind;
        node->layer = uint32_t(number(patch, L"render_layer_id"));
        layer_count_ = std::max(layer_count_, node->layer + 1);
        if (present(patch, L"parent_frame")) node->parent = uint32_t(number(patch, L"parent_frame"));
        node->host = controls::Grid();
        node->host.HorizontalAlignment(xaml::HorizontalAlignment::Left);
        node->host.VerticalAlignment(xaml::VerticalAlignment::Top);
        auto live = node->events.live;
        if (kind == L"Textbox") {
            controls::TextBox editor;
            auto changed = editor.TextChanged([this, id, live](auto const& sender, auto const&) { if (*live) send(L"FormTextboxInput", id, L"text", text(sender.template as<controls::TextBox>().Text())); });
            node->events.revoke.push_back([editor, changed] { editor.TextChanged(changed); });
            auto focus = editor.LostFocus([this, id, live](auto const& sender, auto const&) { if (*live) send(L"FormTextboxChange", id, L"text", text(sender.template as<controls::TextBox>().Text())); });
            node->events.revoke.push_back([editor, focus] { editor.LostFocus(focus); });
            auto selected = editor.SelectionChanged([this, live](auto const&, auto const&) { if (*live && !applying_) interrupt_(event(L"SelectStart", json::JsonObject())); });
            node->events.revoke.push_back([editor, selected] { editor.SelectionChanged(selected); });
            node->control = editor;
        } else if (kind == L"Button") {
            controls::Button button;
            auto clicked = button.Click([this, id, live](auto const&, auto const&) { if (*live) send(L"FormButtonClick", id); });
            node->events.revoke.push_back([button, clicked] { button.Click(clicked); }); node->control = button;
        } else if (kind == L"Checkbox") {
            controls::CheckBox check;
            auto checked = check.Checked([this, id, live](auto const&, auto const&) { if (*live) send(L"FormCheckboxToggle", id, L"state", json::JsonValue::CreateBooleanValue(true)); });
            node->events.revoke.push_back([check, checked] { check.Checked(checked); });
            auto unchecked = check.Unchecked([this, id, live](auto const&, auto const&) { if (*live) send(L"FormCheckboxToggle", id, L"state", json::JsonValue::CreateBooleanValue(false)); });
            node->events.revoke.push_back([check, unchecked] { check.Unchecked(unchecked); });
            node->control = check;
        } else if (kind == L"Text") node->control = controls::TextBlock();
        else if (kind == L"Frame" || kind == L"Scroller") {
            node->children = controls::Canvas();
            if (kind == L"Frame") node->control = node->children;
            else {
                controls::ScrollViewer scroll; scroll.Content(node->children);
                auto changed = scroll.ViewChanged([this, id, live](auto const& sender, auto const&) {
                    if (!*live || applying_) return;
                    auto scroll = sender.template as<controls::ScrollViewer>();
                    json::JsonObject args; args.Insert(L"id", numeric(id));
                    args.Insert(L"scroll_x", numeric(scroll.HorizontalOffset())); args.Insert(L"scroll_y", numeric(scroll.VerticalOffset()));
                    args.Insert(L"presentation_scroll_x", numeric(scroll.HorizontalOffset())); args.Insert(L"presentation_scroll_y", numeric(scroll.VerticalOffset()));
                    interrupt_(event(L"ScrollerPosition", args));
                });
                node->events.revoke.push_back([scroll, changed] { scroll.ViewChanged(changed); }); node->control = scroll;
            }
        } else if (kind == L"EventBlocker") {
            controls::Grid blocker; blocker.Background(media::SolidColorBrush(winrt::Microsoft::UI::Colors::Transparent())); node->control = blocker;
        } else throw std::invalid_argument("Unsupported native control family: " + winrt::to_string(kind));
        xaml::Automation::AutomationProperties::SetAutomationId(node->control, L"pax-node-" + winrt::to_hstring(id));
        node->host.Children().Append(node->control);
        nodes_.emplace(id, std::move(node));
    }

    void update(Node& node, json::JsonObject const& patch) {
        if (patch.HasKey(L"parent_frame")) node.parent = present(patch, L"parent_frame") ? std::optional<uint32_t>(uint32_t(number(patch, L"parent_frame"))) : std::nullopt;
        if (present(patch, L"size_x")) { node.width = std::max(0.0, number(patch, L"size_x")); node.mask_dirty = bool(node.mask); }
        if (present(patch, L"size_y")) { node.height = std::max(0.0, number(patch, L"size_y")); node.mask_dirty = bool(node.mask); }
        if (present(patch, L"transform")) {
            auto t = patch.GetNamedArray(L"transform");
            if (t.Size() != 6) throw std::invalid_argument("Native affine transform requires six values");
            node.world = {t.GetNumberAt(0), t.GetNumberAt(1), t.GetNumberAt(2), t.GetNumberAt(3), t.GetNumberAt(4), t.GetNumberAt(5)};
            if (node.clip_path) node.mask_dirty = true;
        }
        if (present(patch, L"z_index")) controls::Canvas::SetZIndex(node.host, int(number(patch, L"z_index")));
        if (present(patch, L"opacity")) node.host.Opacity(std::clamp(number(patch, L"opacity"), 0.0, 1.0));
        if (auto editor = node.control.try_as<controls::TextBox>()) {
            if (present(patch, L"text") && editor.Text() != patch.GetNamedString(L"text")) editor.Text(patch.GetNamedString(L"text"));
            if (present(patch, L"placeholder")) editor.PlaceholderText(patch.GetNamedString(L"placeholder"));
            if (present(patch, L"is_text_area")) { editor.AcceptsReturn(boolean(patch, L"is_text_area")); editor.TextWrapping(boolean(patch, L"is_text_area") ? xaml::TextWrapping::Wrap : xaml::TextWrapping::NoWrap); }
            if (present(patch, L"style")) style(editor, patch.GetNamedObject(L"style"));
            if (present(patch, L"focus_on_mount")) node.focus_pending = boolean(patch, L"focus_on_mount");
        } else if (auto button = node.control.try_as<controls::Button>()) {
            if (present(patch, L"content")) button.Content(winrt::box_value(patch.GetNamedString(L"content")));
            if (present(patch, L"style")) style(button, patch.GetNamedObject(L"style"));
        } else if (auto check = node.control.try_as<controls::CheckBox>()) {
            if (present(patch, L"checked")) check.IsChecked(boolean(patch, L"checked"));
        } else if (auto label = node.control.try_as<controls::TextBlock>()) {
            if (boolean(patch, L"editable") || boolean(patch, L"markdown")) throw std::invalid_argument("Rich/editable Text is not yet implemented by this Windows host");
            if (present(patch, L"content")) label.Text(patch.GetNamedString(L"content"));
            if (present(patch, L"selectable")) label.IsTextSelectionEnabled(boolean(patch, L"selectable"));
            if (present(patch, L"wrap")) label.TextWrapping(boolean(patch, L"wrap") ? xaml::TextWrapping::Wrap : xaml::TextWrapping::NoWrap);
            if (present(patch, L"style")) style(label, patch.GetNamedObject(L"style"));
            if (present(patch, L"measure_generation")) node.measure_generation = uint64_t(number(patch, L"measure_generation"));
        } else if (auto scroll = node.control.try_as<controls::ScrollViewer>()) {
            if (present(patch, L"size_inner_pane_x")) node.children.Width(number(patch, L"size_inner_pane_x"));
            if (present(patch, L"size_inner_pane_y")) node.children.Height(number(patch, L"size_inner_pane_y"));
            if (present(patch, L"scroll_enabled_x")) { bool enabled = boolean(patch, L"scroll_enabled_x"); scroll.HorizontalScrollMode(enabled ? controls::ScrollMode::Enabled : controls::ScrollMode::Disabled); scroll.HorizontalScrollBarVisibility(enabled ? controls::ScrollBarVisibility::Auto : controls::ScrollBarVisibility::Disabled); }
            if (present(patch, L"scroll_enabled_y")) { bool enabled = boolean(patch, L"scroll_enabled_y"); scroll.VerticalScrollMode(enabled ? controls::ScrollMode::Enabled : controls::ScrollMode::Disabled); scroll.VerticalScrollBarVisibility(enabled ? controls::ScrollBarVisibility::Auto : controls::ScrollBarVisibility::Disabled); }
            if (present(patch, L"scroll_x") || present(patch, L"scroll_y")) scroll.ChangeView(number(patch, L"scroll_x", scroll.HorizontalOffset()), number(patch, L"scroll_y", scroll.VerticalOffset()), nullptr, true);
            if (present(patch, L"content_layer_id")) layer_count_ = std::max(layer_count_, uint32_t(number(patch, L"content_layer_id")) + 1);
        }
        if (auto control = node.control.try_as<controls::Control>()) {
            if (present(patch, L"background")) control.Background(color(patch.GetNamedValue(L"background")));
            if (present(patch, L"stroke_color")) control.BorderBrush(color(patch.GetNamedValue(L"stroke_color")));
            if (present(patch, L"stroke_width")) { auto width = number(patch, L"stroke_width"); control.BorderThickness(xaml::Thickness{width, width, width, width}); }
        }
        if (present(patch, L"clip_path")) {
            auto value = patch.GetNamedString(L"clip_path");
            if (value.empty()) node.clip_path.reset(); else node.clip_path = masks_.parse(value);
            node.mask_dirty = true;
        }
        if (present(patch, L"clip_content")) node.clip_content = boolean(patch, L"clip_content");
    }

    void update_mask(Node& node) {
        using xaml::Hosting::ElementCompositionPreview;
        auto source = ElementCompositionPreview::GetElementVisual(node.control);
        auto holes = node.holes;
        if (node.clip_path && node.width > 0 && node.height > 0) {
            // FramePatch.clip_path is world-space; NativeMaskUpdate coverage
            // is already local. Keep these two established contracts distinct.
            auto inverse = relative(media::Matrix{1, 0, 0, 1, 0, 0}, node.world);
            auto local_clip = masks_.transform(*node.clip_path, D2D1::Matrix3x2F(float(inverse.M11), float(inverse.M12),
                float(inverse.M21), float(inverse.M22), float(inverse.OffsetX), float(inverse.OffsetY)));
            MaskHole outside;
            outside.shape = masks_.combine(masks_.rectangle(float(node.width), float(node.height)), local_clip, D2D1_COMBINE_MODE_EXCLUDE);
            holes.push_back(std::move(outside));
        }
        if (holes.empty() || node.width <= 0 || node.height <= 0) {
            node.mask_events.clear();
            source.Opacity(1); ElementCompositionPreview::SetElementChildVisual(node.host, nullptr);
            if (node.mask_input) { uint32_t index = 0; if (node.host.Children().IndexOf(node.mask_input, index)) node.host.Children().RemoveAt(index); }
            node.mask_input = nullptr; node.mask = nullptr; node.capture = nullptr; node.masked = nullptr;
            node.mask_dirty = false; return;
        }
        node.mask = masks_.draw(float(node.width), float(node.height), dpr_, holes, node.mask);
        if (!node.capture) { node.capture = compositor_.CreateVisualSurface(); node.capture.SourceVisual(source); }
        node.capture.SourceSize({float(node.width), float(node.height)});
        auto brush = compositor_.CreateMaskBrush();
        brush.Source(compositor_.CreateSurfaceBrush(node.capture)); brush.Mask(compositor_.CreateSurfaceBrush(node.mask));
        if (!node.masked) node.masked = compositor_.CreateSpriteVisual();
        node.masked.Size({float(node.width), float(node.height)}); node.masked.Brush(brush);
        source.Opacity(0); ElementCompositionPreview::SetElementChildVisual(node.host, node.masked);
        if (!node.mask_input) { node.mask_input = controls::Canvas(); node.host.Children().Append(node.mask_input); }
        node.mask_events.clear();
        node.mask_input.Children().Clear();
        node.mask_input.Width(node.width); node.mask_input.Height(node.height);
        for (auto const& hole : holes) {
            if (hole.opacity < 1) continue;
            xaml::Shapes::Path path; path.Data(hole.shape.input);
            path.Fill(media::SolidColorBrush(winrt::Microsoft::UI::Colors::Transparent()));
            path.Stretch(media::Stretch::None); wire_pointer(path, node.mask_events, true);
            xaml::UIElement element = path;
            node.mask_input.Children().Append(element);
        }
        node.mask_dirty = false;
    }

public:
    NativeScene(Interrupt interrupt, std::function<winrt::hstring(json::JsonObject const&)> local_font,
                std::function<void(winrt::hstring const&)> load_image)
        : interrupt_(std::move(interrupt)), local_font_(std::move(local_font)), load_image_(std::move(load_image)) {
        root_.Background(media::SolidColorBrush(winrt::Microsoft::UI::Colors::Transparent()));
        focus_parking_.Opacity(0); focus_parking_.Width(1); focus_parking_.Height(1);
        root_.Children().Append(focus_parking_); wire_pointer(root_, root_events_);
    }
    ~NativeScene() { root_events_.retire(); for (auto& [id, node] : nodes_) { node->events.retire(); node->mask_events.retire(); } }
    controls::Canvas root() const { return root_; }
    uint32_t layer_count() const { return layer_count_; }
    void initialize(ID3D11Device* device) {
        compositor_ = xaml::Hosting::ElementCompositionPreview::GetElementVisual(root_).Compositor();
        masks_.initialize(compositor_, device);
    }
    controls::Canvas layer(uint32_t id, std::optional<uint32_t> owner) {
        auto parent = owner ? require(*owner).children : root_;
        if (!parent) throw std::invalid_argument("Canvas owner is not a native container");
        auto [found, inserted] = layers_.try_emplace(id);
        auto& layer = found->second;
        if (inserted) {
            layer.canvas.IsHitTestVisible(false);
            // Reparenting must not turn insertion order into vector z-order.
            controls::Canvas::SetZIndex(layer.canvas, -1000000 + int(id));
        }
        if (layer.parent != parent) {
            if (layer.parent) { uint32_t index = 0; if (layer.parent.Children().IndexOf(layer.canvas, index)) layer.parent.Children().RemoveAt(index); }
            parent.Children().Append(layer.canvas); layer.parent = parent;
        }
        return layer.canvas;
    }
    void resize(double width, double height, float dpr) {
        root_.Width(width); root_.Height(height);
        if (dpr != dpr_) { dpr_ = dpr; for (auto& [id, node] : nodes_) node->mask_dirty = bool(node->mask); }
    }
    void apply(json::JsonArray const& messages) {
        applying_ = true;
        try {
            for (auto const& message : messages) {
                auto object = message.GetObject();
                if (object.Size() != 1) throw std::invalid_argument("Invalid native message envelope");
                auto pair = object.First().Current(); std::wstring name(pair.Key());
                if (name == L"ImageLoad") { auto patch = pair.Value().GetObject(); if (present(patch, L"path")) load_image_(patch.GetNamedString(L"path")); continue; }
                if (name == L"ShrinkLayersTo") {
                    layer_count_ = uint32_t(pair.Value().GetNumber());
                    for (auto it = layers_.lower_bound(layer_count_); it != layers_.end();) {
                        uint32_t index = 0;
                        if (it->second.parent && it->second.parent.Children().IndexOf(it->second.canvas, index))
                            it->second.parent.Children().RemoveAt(index);
                        it = layers_.erase(it);
                    }
                    continue;
                }
                if (name == L"NativeCullUpdate") continue; // Advisory only: preserve native focus, editing and a11y identity.
                if (name == L"NativeMaskUpdate") {
                    auto patch = pair.Value().GetObject(); auto& node = require(uint32_t(number(patch, L"id")));
                    node.holes = masks_.parse_holes(patch.GetNamedArray(L"entries")); node.mask_dirty = true; continue;
                }
                if (name.ends_with(L"Create")) create(name.substr(0, name.size() - 6), pair.Value().GetObject());
                else if (name.ends_with(L"Update")) { auto patch = pair.Value().GetObject(); update(require(uint32_t(number(patch, L"id"))), patch); }
                else if (name.ends_with(L"Delete")) {
                    uint32_t id = uint32_t(pair.Value().GetNumber()); auto& node = require(id); detach(node); nodes_.erase(id);
                } else throw std::invalid_argument("Unsupported Windows native message: " + winrt::to_string(name));
            }
            for (auto& [id, node] : nodes_) {
                placement(*node); if (node->mask_dirty) update_mask(*node);
                // A create/update patch can arrive before XAML has attached the
                // editor. Retry requested mount focus after placement/layout.
                if (node->focus_pending) if (auto editor = node->control.try_as<controls::TextBox>())
                    if (editor.Focus(xaml::FocusState::Programmatic)) node->focus_pending = false;
            }
            applying_ = false;
            for (auto& [id, node] : nodes_) if (node->measure_generation) {
                node->control.Measure({float(node->width > 0 ? node->width : 1000000), 1000000});
                auto size = node->control.DesiredSize(); json::JsonObject args;
                args.Insert(L"id", numeric(id)); args.Insert(L"generation", numeric(double(*node->measure_generation)));
                args.Insert(L"width", numeric(size.Width)); args.Insert(L"height", numeric(size.Height));
                node->measure_generation.reset(); interrupt_(event(L"TextMeasurementResponse", args));
            }
        } catch (...) { applying_ = false; throw; }
    }
    void close() {
        applying_ = true;
        root_events_.retire();
        focus_parking_.Focus(xaml::FocusState::Programmatic);
        for (auto& [id, node] : nodes_) {
            xaml::Hosting::ElementCompositionPreview::SetElementChildVisual(node->host, nullptr);
            detach(*node);
        }
        nodes_.clear(); layers_.clear(); root_.Children().Clear();
    }
};
} // namespace pax::windows

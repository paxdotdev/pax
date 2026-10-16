#include <gtk/gtk.h>
#include <vulkan/vulkan.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>

/* Allocation/export preflight only. No image pixels, GDK import, WGPU adapter,
 * external synchronization or presentation are exercised by this program. */
#define PROBE_WIDTH 64
#define PROBE_HEIGHT 64
#define PROBE_USAGE (VK_IMAGE_USAGE_SAMPLED_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT)
static bool operational_error;
static uint32_t instance_api_version = VK_API_VERSION_1_1;
static GdkDmabufFormats *display_formats;

static const char *boolean(bool value) { return value ? "true" : "false"; }
static void string(const char *value) {
    if (!value) { fputs("null", stdout); return; }
    putchar('"');
    for (const unsigned char *p = (const unsigned char *)value; *p; p++) {
        if (*p == '"' || *p == '\\') { putchar('\\'); putchar(*p); }
        else if (*p < 32) printf("\\u%04x", *p);
        else putchar(*p);
    }
    putchar('"');
}
static void uuid(const uint8_t bytes[VK_UUID_SIZE]) {
    putchar('"');
    for (unsigned i = 0; i < VK_UUID_SIZE; i++) printf("%02x", bytes[i]);
    putchar('"');
}
static void *array(uint32_t count, size_t size) {
    void *result = calloc(count ? count : 1, size);
    if (!result) { fputs("preflight allocation failed\n", stderr); exit(1); }
    return result;
}
static bool extension(const VkExtensionProperties *extensions, uint32_t count, const char *name) {
    for (uint32_t i = 0; i < count; i++) if (!strcmp(extensions[i].extensionName, name)) return true;
    return false;
}
static bool display_supports(uint32_t fourcc, uint64_t modifier) {
    if (!display_formats) return false;
    for (gsize i = 0; i < gdk_dmabuf_formats_get_n_formats(display_formats); i++) {
        guint32 candidate_fourcc;
        guint64 candidate_modifier;
        gdk_dmabuf_formats_get_format(display_formats, i, &candidate_fourcc, &candidate_modifier);
        if (candidate_fourcc == fourcc && candidate_modifier == modifier) return true;
    }
    return false;
}
static const char *device_class(VkPhysicalDeviceType type) {
    switch (type) {
        case VK_PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU: return "integrated";
        case VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU: return "discrete";
        case VK_PHYSICAL_DEVICE_TYPE_VIRTUAL_GPU: return "virtual";
        case VK_PHYSICAL_DEVICE_TYPE_CPU: return "cpu";
        default: return "other_or_unknown";
    }
}

typedef struct {
    bool attempted, exported;
    const char *stage;
    VkResult result;
    uint64_t allocation_size, actual_modifier;
    VkSubresourceLayout planes[4];
} ExportResult;

static ExportResult export_image(VkPhysicalDevice physical, VkDevice device, VkFormat format,
        VkImageTiling tiling, uint64_t modifier, uint32_t planes) {
    ExportResult output = {.attempted = true, .stage = "create_image", .result = VK_SUCCESS};
    VkImage image = VK_NULL_HANDLE;
    VkDeviceMemory memory = VK_NULL_HANDLE;
    int fd = -1;
    VkImageDrmFormatModifierListCreateInfoEXT modifiers = {
        .sType = VK_STRUCTURE_TYPE_IMAGE_DRM_FORMAT_MODIFIER_LIST_CREATE_INFO_EXT,
        .drmFormatModifierCount = 1, .pDrmFormatModifiers = &modifier};
    VkExternalMemoryImageCreateInfo external = {.sType = VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO,
        .pNext = tiling == VK_IMAGE_TILING_DRM_FORMAT_MODIFIER_EXT ? &modifiers : NULL,
        .handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT};
    VkImageCreateInfo create = {.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO, .pNext = &external,
        .imageType = VK_IMAGE_TYPE_2D, .format = format, .extent = {PROBE_WIDTH, PROBE_HEIGHT, 1},
        .mipLevels = 1, .arrayLayers = 1, .samples = VK_SAMPLE_COUNT_1_BIT,
        .tiling = tiling, .usage = PROBE_USAGE, .sharingMode = VK_SHARING_MODE_EXCLUSIVE,
        .initialLayout = VK_IMAGE_LAYOUT_UNDEFINED};
    output.result = vkCreateImage(device, &create, NULL, &image);
    if (output.result != VK_SUCCESS) goto done;
    VkMemoryRequirements requirements;
    vkGetImageMemoryRequirements(device, image, &requirements);
    output.allocation_size = requirements.size;
    VkPhysicalDeviceMemoryProperties memory_properties;
    vkGetPhysicalDeviceMemoryProperties(physical, &memory_properties);
    uint32_t memory_type = 0;
    while (memory_type < memory_properties.memoryTypeCount && !(requirements.memoryTypeBits & (1u << memory_type))) memory_type++;
    if (memory_type == memory_properties.memoryTypeCount) {
        output.stage = "no_memory_type"; output.result = VK_ERROR_FEATURE_NOT_PRESENT; goto done;
    }
    VkMemoryDedicatedAllocateInfo dedicated = {.sType = VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO, .image = image};
    VkExportMemoryAllocateInfo export = {.sType = VK_STRUCTURE_TYPE_EXPORT_MEMORY_ALLOCATE_INFO,
        .pNext = &dedicated, .handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT};
    VkMemoryAllocateInfo allocate = {.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO, .pNext = &export,
        .allocationSize = requirements.size, .memoryTypeIndex = memory_type};
    output.stage = "allocate_memory";
    output.result = vkAllocateMemory(device, &allocate, NULL, &memory);
    if (output.result != VK_SUCCESS) goto done;
    output.stage = "bind_image_memory";
    output.result = vkBindImageMemory(device, image, memory, 0);
    if (output.result != VK_SUCCESS) goto done;
    if (tiling == VK_IMAGE_TILING_DRM_FORMAT_MODIFIER_EXT) {
        PFN_vkGetImageDrmFormatModifierPropertiesEXT get_modifier =
            (PFN_vkGetImageDrmFormatModifierPropertiesEXT)vkGetDeviceProcAddr(device, "vkGetImageDrmFormatModifierPropertiesEXT");
        if (!get_modifier) { operational_error = true; output.stage = "missing_modifier_entrypoint"; goto done; }
        VkImageDrmFormatModifierPropertiesEXT properties = {.sType = VK_STRUCTURE_TYPE_IMAGE_DRM_FORMAT_MODIFIER_PROPERTIES_EXT};
        output.stage = "get_actual_modifier";
        output.result = get_modifier(device, image, &properties);
        if (output.result != VK_SUCCESS) goto done;
        output.actual_modifier = properties.drmFormatModifier;
        if (output.actual_modifier != modifier) {
            output.stage = "modifier_mismatch"; operational_error = true; goto done;
        }
    }
    for (uint32_t plane = 0; plane < planes; plane++) {
        VkImageSubresource subresource = {.aspectMask = tiling == VK_IMAGE_TILING_LINEAR
            ? VK_IMAGE_ASPECT_COLOR_BIT : (VkImageAspectFlags)(VK_IMAGE_ASPECT_MEMORY_PLANE_0_BIT_EXT << plane)};
        vkGetImageSubresourceLayout(device, image, &subresource, &output.planes[plane]);
    }
    PFN_vkGetMemoryFdKHR get_fd = (PFN_vkGetMemoryFdKHR)vkGetDeviceProcAddr(device, "vkGetMemoryFdKHR");
    if (!get_fd) { operational_error = true; output.stage = "missing_memory_fd_entrypoint"; goto done; }
    VkMemoryGetFdInfoKHR info = {.sType = VK_STRUCTURE_TYPE_MEMORY_GET_FD_INFO_KHR,
        .memory = memory, .handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT};
    output.stage = "get_memory_fd";
    output.result = get_fd(device, &info, &fd);
    if (output.result != VK_SUCCESS) goto done;
    struct stat status;
    if (fd < 0 || fstat(fd, &status) != 0) {
        output.stage = "invalid_exported_fd"; operational_error = true; goto done;
    }
    output.exported = true;
    output.stage = "fd_exported_and_closed";
done:
    if (fd >= 0) close(fd);
    if (image) vkDestroyImage(device, image, NULL);
    if (memory) vkFreeMemory(device, memory, NULL);
    /* An attempted allocation has already passed the exact capability query.
     * Driver/allocation failures are operational errors, not absent support. */
    if (output.result != VK_SUCCESS) operational_error = true;
    return output;
}

/* Returns true only for an actual FD export intersecting the display's formats.
 * Even this result says nothing about GDK import or device identity matching. */
static bool candidate(VkPhysicalDevice physical, VkDevice device, bool can_query, VkFormat format,
        uint32_t fourcc, VkImageTiling tiling, uint64_t modifier, uint32_t planes,
        VkFormatFeatureFlags tiling_features, bool *current_exporter_candidate) {
    bool drm = tiling == VK_IMAGE_TILING_DRM_FORMAT_MODIFIER_EXT;
    bool common = display_supports(fourcc, modifier);
    VkPhysicalDeviceImageDrmFormatModifierInfoEXT modifier_info = {
        .sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_IMAGE_DRM_FORMAT_MODIFIER_INFO_EXT,
        .drmFormatModifier = modifier, .sharingMode = VK_SHARING_MODE_EXCLUSIVE};
    VkPhysicalDeviceExternalImageFormatInfo external = {
        .sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_EXTERNAL_IMAGE_FORMAT_INFO,
        .pNext = drm ? &modifier_info : NULL, .handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT};
    VkPhysicalDeviceImageFormatInfo2 info = {.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_IMAGE_FORMAT_INFO_2,
        .pNext = &external, .format = format, .type = VK_IMAGE_TYPE_2D, .tiling = tiling, .usage = PROBE_USAGE};
    VkExternalImageFormatProperties external_properties = {.sType = VK_STRUCTURE_TYPE_EXTERNAL_IMAGE_FORMAT_PROPERTIES};
    VkImageFormatProperties2 properties = {.sType = VK_STRUCTURE_TYPE_IMAGE_FORMAT_PROPERTIES_2, .pNext = &external_properties};
    VkResult result = can_query ? vkGetPhysicalDeviceImageFormatProperties2(physical, &info, &properties) : VK_ERROR_EXTENSION_NOT_PRESENT;
    VkExternalMemoryProperties memory = external_properties.externalMemoryProperties;
    bool exportable = result == VK_SUCCESS && (memory.externalMemoryFeatures & VK_EXTERNAL_MEMORY_FEATURE_EXPORTABLE_BIT)
        && (memory.compatibleHandleTypes & VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT);
    bool dimensions = result == VK_SUCCESS && properties.imageFormatProperties.maxExtent.width >= PROBE_WIDTH
        && properties.imageFormatProperties.maxExtent.height >= PROBE_HEIGHT
        && (properties.imageFormatProperties.sampleCounts & VK_SAMPLE_COUNT_1_BIT);
    bool usage = !drm || ((tiling_features & (VK_FORMAT_FEATURE_SAMPLED_IMAGE_BIT | VK_FORMAT_FEATURE_TRANSFER_DST_BIT))
        == (VK_FORMAT_FEATURE_SAMPLED_IMAGE_BIT | VK_FORMAT_FEATURE_TRANSFER_DST_BIT));
    ExportResult exported = {.stage = "not_attempted"};
    if (device && exportable && dimensions && usage && planes >= 1 && planes <= 4 && (!drm || common))
        exported = export_image(physical, device, format, tiling, modifier, planes);
    bool current = !drm && format == VK_FORMAT_R8G8B8A8_UNORM && exported.exported && common;
    *current_exporter_candidate |= current;
    printf("{\"format\":%u,\"fourcc\":%u,\"tiling\":", format, fourcc);
    string(drm ? "drm_modifier" : "linear");
    printf(",\"modifier\":\"0x%016" PRIx64 "\",\"plane_count\":%u,\"tiling_features\":%u,"
        "\"gdk_modifier_match\":%s,\"query_result\":%d,\"external_memory_features\":%u,"
        "\"compatible_handle_types\":%u,\"export_from_imported_handle_types\":%u,"
        "\"exportable\":%s,\"importable\":%s,\"dedicated_only\":%s,\"current_exporter_candidate\":%s,"
        "\"export\":{\"attempted\":%s,\"succeeded\":%s,\"stage\":",
        modifier, planes, tiling_features, boolean(common), result, memory.externalMemoryFeatures,
        memory.compatibleHandleTypes, memory.exportFromImportedHandleTypes, boolean(exportable),
        boolean(memory.externalMemoryFeatures & VK_EXTERNAL_MEMORY_FEATURE_IMPORTABLE_BIT),
        boolean(memory.externalMemoryFeatures & VK_EXTERNAL_MEMORY_FEATURE_DEDICATED_ONLY_BIT), boolean(current),
        boolean(exported.attempted), boolean(exported.exported));
    string(exported.stage);
    printf(",\"vk_result\":%d,\"allocation_bytes\":%" PRIu64 ",\"actual_modifier\":",
        exported.result, exported.allocation_size);
    if (exported.exported) printf("\"0x%016" PRIx64 "\"", exported.actual_modifier);
    else fputs("null", stdout);
    fputs(",\"contents_written\":false,\"planes\":[", stdout);
    if (exported.exported) for (uint32_t p = 0; p < planes; p++) {
        if (p) putchar(',');
        printf("{\"offset\":%" PRIu64 ",\"row_pitch\":%" PRIu64 ",\"size\":%" PRIu64 "}",
            exported.planes[p].offset, exported.planes[p].rowPitch, exported.planes[p].size);
    }
    fputs("]}}", stdout);
    return common && exported.exported && drm;
}

static bool inspect_device(VkPhysicalDevice physical, unsigned index, bool *any_current) {
    VkPhysicalDeviceProperties basic;
    vkGetPhysicalDeviceProperties(physical, &basic);
    uint32_t extension_count = 0;
    VkResult extension_result = vkEnumerateDeviceExtensionProperties(physical, NULL, &extension_count, NULL);
    VkExtensionProperties *extensions = array(extension_count, sizeof(*extensions));
    if (extension_result == VK_SUCCESS)
        extension_result = vkEnumerateDeviceExtensionProperties(physical, NULL, &extension_count, extensions);
    if (extension_result != VK_SUCCESS) { extension_count = 0; operational_error = true; }
    bool api11 = basic.apiVersion >= VK_API_VERSION_1_1;
    bool memory_fd = extension(extensions, extension_count, VK_KHR_EXTERNAL_MEMORY_FD_EXTENSION_NAME);
    bool dma_buf = extension(extensions, extension_count, VK_EXT_EXTERNAL_MEMORY_DMA_BUF_EXTENSION_NAME);
    bool drm = extension(extensions, extension_count, VK_EXT_IMAGE_DRM_FORMAT_MODIFIER_EXTENSION_NAME);
    bool semaphore_fd = extension(extensions, extension_count, VK_KHR_EXTERNAL_SEMAPHORE_FD_EXTENSION_NAME);
    bool foreign_queue = extension(extensions, extension_count, VK_EXT_QUEUE_FAMILY_FOREIGN_EXTENSION_NAME);
    bool core12 = basic.apiVersion >= VK_API_VERSION_1_2 && instance_api_version >= VK_API_VERSION_1_2;
    bool driver_info = api11 && (core12 || extension(extensions, extension_count, VK_KHR_DRIVER_PROPERTIES_EXTENSION_NAME));
    VkPhysicalDeviceDriverProperties driver = {.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_DRIVER_PROPERTIES};
    VkPhysicalDeviceIDProperties ids = {.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_ID_PROPERTIES,
        .pNext = driver_info ? &driver : NULL};
    VkPhysicalDeviceProperties2 properties = {.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2, .pNext = &ids};
    if (api11) vkGetPhysicalDeviceProperties2(physical, &properties);
    bool physical_gpu = (basic.deviceType == VK_PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU || basic.deviceType == VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU)
        && driver.driverID != VK_DRIVER_ID_MESA_LLVMPIPE && driver.driverID != VK_DRIVER_ID_GOOGLE_SWIFTSHADER;
    VkPhysicalDeviceExternalSemaphoreInfo semaphore_info = {.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_EXTERNAL_SEMAPHORE_INFO,
        .handleType = VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_SYNC_FD_BIT};
    VkExternalSemaphoreProperties semaphore = {.sType = VK_STRUCTURE_TYPE_EXTERNAL_SEMAPHORE_PROPERTIES};
    if (api11 && semaphore_fd) vkGetPhysicalDeviceExternalSemaphoreProperties(physical, &semaphore_info, &semaphore);
    bool sync_export = semaphore_fd && (semaphore.externalSemaphoreFeatures & VK_EXTERNAL_SEMAPHORE_FEATURE_EXPORTABLE_BIT)
        && (semaphore.compatibleHandleTypes & VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_SYNC_FD_BIT);
    bool sync_import = semaphore_fd && (semaphore.externalSemaphoreFeatures & VK_EXTERNAL_SEMAPHORE_FEATURE_IMPORTABLE_BIT);
    uint32_t queue_count = 0;
    vkGetPhysicalDeviceQueueFamilyProperties(physical, &queue_count, NULL);
    VkQueueFamilyProperties *queues = array(queue_count, sizeof(*queues));
    vkGetPhysicalDeviceQueueFamilyProperties(physical, &queue_count, queues);
    uint32_t queue_family = 0;
    while (queue_family < queue_count && !(queues[queue_family].queueCount && (queues[queue_family].queueFlags & VK_QUEUE_GRAPHICS_BIT))) queue_family++;
    free(queues);
    VkDevice device = VK_NULL_HANDLE;
    VkResult device_result = VK_ERROR_EXTENSION_NOT_PRESENT;
    bool modifier_dependencies = core12
        || extension(extensions, extension_count, VK_KHR_IMAGE_FORMAT_LIST_EXTENSION_NAME);
    bool enable_drm = drm && modifier_dependencies;
    if (api11 && memory_fd && dma_buf && queue_family < queue_count) {
        const char *enabled[4] = {VK_KHR_EXTERNAL_MEMORY_FD_EXTENSION_NAME, VK_EXT_EXTERNAL_MEMORY_DMA_BUF_EXTENSION_NAME};
        uint32_t enabled_count = 2;
        if (enable_drm) {
            enabled[enabled_count++] = VK_EXT_IMAGE_DRM_FORMAT_MODIFIER_EXTENSION_NAME;
            if (!core12) enabled[enabled_count++] = VK_KHR_IMAGE_FORMAT_LIST_EXTENSION_NAME;
        }
        float priority = 1;
        VkDeviceQueueCreateInfo queue = {.sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
            .queueFamilyIndex = queue_family, .queueCount = 1, .pQueuePriorities = &priority};
        VkDeviceCreateInfo create = {.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO, .queueCreateInfoCount = 1,
            .pQueueCreateInfos = &queue, .enabledExtensionCount = enabled_count, .ppEnabledExtensionNames = enabled};
        device_result = vkCreateDevice(physical, &create, NULL, &device);
        if (device_result != VK_SUCCESS) operational_error = true;
    }
    printf("{\"index\":%u,\"name\":", index); string(basic.deviceName);
    fputs(",\"device_class\":", stdout); string(device_class(basic.deviceType));
    printf(",\"device_type\":%u,\"physical_gpu_candidate\":%s,\"vendor_id\":%u,\"device_id\":%u,"
        "\"api_version\":%u,\"driver_version\":%u,\"driver_properties_available\":%s,\"driver_id\":%u,\"driver_name\":",
        basic.deviceType, boolean(physical_gpu), basic.vendorID, basic.deviceID, basic.apiVersion, basic.driverVersion,
        boolean(driver_info && api11), driver.driverID);
    string(driver_info ? driver.driverName : NULL);
    fputs(",\"driver_info\":", stdout); string(driver_info ? driver.driverInfo : NULL);
    fputs(",\"device_uuid\":", stdout); uuid(ids.deviceUUID);
    fputs(",\"driver_uuid\":", stdout); uuid(ids.driverUUID);
    printf(",\"extensions\":{\"external_memory_fd\":%s,\"external_memory_dma_buf\":%s,\"image_drm_format_modifier\":%s,"
        "\"external_semaphore_fd\":%s,\"queue_family_foreign\":%s},\"extension_query_result\":%d,\"device_create_result\":%d,"
        "\"sync_fd\":{\"queried\":%s,\"exportable\":%s,\"importable\":%s,\"features\":%u,"
        "\"compatible_handle_types\":%u,\"export_from_imported_handle_types\":%u,\"handoff_tested\":false},\"images\":[",
        boolean(memory_fd), boolean(dma_buf), boolean(drm), boolean(semaphore_fd), boolean(foreign_queue), extension_result, device_result,
        boolean(api11 && semaphore_fd), boolean(sync_export), boolean(sync_import), semaphore.externalSemaphoreFeatures,
        semaphore.compatibleHandleTypes, semaphore.exportFromImportedHandleTypes);
    const VkFormat formats[] = {VK_FORMAT_R8G8B8A8_UNORM, VK_FORMAT_B8G8R8A8_UNORM};
    const uint32_t fourccs[] = {0x34324241, 0x34325241}; /* DRM AB24 / AR24 */
    bool common_export = false, current = false, first = true;
    for (unsigned f = 0; f < 2; f++) {
        if (!first) putchar(',');
        first = false;
        candidate(physical, device, api11 && memory_fd && dma_buf, formats[f], fourccs[f],
            VK_IMAGE_TILING_LINEAR, 0, 1, 0, &current);
        if (api11 && drm) {
            VkDrmFormatModifierPropertiesListEXT list = {.sType = VK_STRUCTURE_TYPE_DRM_FORMAT_MODIFIER_PROPERTIES_LIST_EXT};
            VkFormatProperties2 format_properties = {.sType = VK_STRUCTURE_TYPE_FORMAT_PROPERTIES_2, .pNext = &list};
            vkGetPhysicalDeviceFormatProperties2(physical, formats[f], &format_properties);
            list.pDrmFormatModifierProperties = array(list.drmFormatModifierCount, sizeof(*list.pDrmFormatModifierProperties));
            vkGetPhysicalDeviceFormatProperties2(physical, formats[f], &format_properties);
            for (uint32_t m = 0; m < list.drmFormatModifierCount; m++) {
                VkDrmFormatModifierPropertiesEXT modifier = list.pDrmFormatModifierProperties[m];
                putchar(',');
                common_export |= candidate(physical, enable_drm ? device : VK_NULL_HANDLE, memory_fd && dma_buf,
                    formats[f], fourccs[f], VK_IMAGE_TILING_DRM_FORMAT_MODIFIER_EXT, modifier.drmFormatModifier,
                    modifier.drmFormatModifierPlaneCount, modifier.drmFormatModifierTilingFeatures, &current);
            }
            free(list.pDrmFormatModifierProperties);
        }
    }
    bool eligible = physical_gpu && common_export && sync_export && sync_import;
    printf("],\"current_exporter_candidate\":%s,\"common_modifier_fd_export\":%s,\"preflight_eligible\":%s,\"blocked_reasons\":[",
        boolean(current), boolean(common_export), boolean(eligible));
    first = true;
#define REASON(condition, value) do { if (condition) { if (!first) putchar(','); string(value); first = false; } } while (0)
    REASON(!physical_gpu, "not_a_physical_integrated_or_discrete_gpu");
    REASON(!api11, "vulkan_1_1_required");
    REASON(!display_formats, "gdk_display_unavailable");
    REASON(!common_export, "no_actual_dma_buf_export_with_common_drm_modifier");
    REASON(!sync_export, "sync_fd_export_not_supported");
    REASON(!sync_import, "sync_fd_import_not_supported_for_reuse_baseline");
#undef REASON
    fputs("]}", stdout);
    *any_current |= current && physical_gpu;
    if (device) vkDestroyDevice(device, NULL);
    free(extensions);
    return eligible;
}

int main(void) {
    GdkDisplay *display = gtk_init_check() ? gdk_display_get_default() : NULL;
    if (display) display_formats = gdk_display_get_dmabuf_formats(display);
    fputs("{\"schema_version\":1,\"probe\":\"pax-linux-hardware-preflight\","
        "\"baseline\":\"physical_gpu_dma_buf_modifier_bidirectional_sync_fd_for_future_reuse\",\"gdk\":{\"display_open\":", stdout);
    fputs(boolean(display != NULL), stdout);
    fputs(",\"backend\":", stdout); string(display ? G_OBJECT_TYPE_NAME(display) : NULL);
    fputs(",\"display_name\":", stdout); string(display ? gdk_display_get_name(display) : NULL);
    fputs(",\"dmabuf_formats\":[", stdout);
    if (display_formats) for (gsize i = 0; i < gdk_dmabuf_formats_get_n_formats(display_formats); i++) {
        guint32 fourcc; guint64 modifier;
        gdk_dmabuf_formats_get_format(display_formats, i, &fourcc, &modifier);
        if (i) putchar(',');
        printf("{\"fourcc\":%u,\"modifier\":\"0x%016" PRIx64 "\"}", fourcc, modifier);
    }
    fputs("]},\"vulkan\":{", stdout);
    PFN_vkEnumerateInstanceVersion get_instance_version =
        (PFN_vkEnumerateInstanceVersion)vkGetInstanceProcAddr(VK_NULL_HANDLE, "vkEnumerateInstanceVersion");
    uint32_t loader_version = VK_API_VERSION_1_0;
    if (get_instance_version && get_instance_version(&loader_version) == VK_SUCCESS && loader_version >= VK_API_VERSION_1_2)
        instance_api_version = VK_API_VERSION_1_2;
    VkApplicationInfo app = {.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO,
        .pApplicationName = "Pax Linux hardware preflight", .apiVersion = instance_api_version};
    VkInstanceCreateInfo create = {.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO, .pApplicationInfo = &app};
    VkInstance instance = VK_NULL_HANDLE;
    VkResult result = vkCreateInstance(&create, NULL, &instance);
    printf("\"instance_api_version\":%u,\"instance_result\":%d,\"devices\":[", app.apiVersion, result);
    bool any_eligible = false, any_current = false;
    if (result == VK_SUCCESS) {
        uint32_t count = 0;
        result = vkEnumeratePhysicalDevices(instance, &count, NULL);
        VkPhysicalDevice *devices = array(count, sizeof(*devices));
        if (result == VK_SUCCESS) result = vkEnumeratePhysicalDevices(instance, &count, devices);
        if (result != VK_SUCCESS) operational_error = true;
        else for (uint32_t d = 0; d < count; d++) {
            if (d) putchar(',');
            any_eligible |= inspect_device(devices[d], d, &any_current);
        }
        free(devices);
        vkDestroyInstance(instance, NULL);
    } else operational_error = true;
    int status = operational_error ? 1 : any_eligible ? 0 : 2;
    printf("]},\"operational_error\":%s,\"inventory_result\":%d,\"preflight_eligible\":%s,"
        "\"current_exporter_candidate\":%s,\"exit_status\":%d,"
        "\"qualification\":{\"wgpu_pax_rendering_tested\":false,\"gdk_import_tested\":false,"
        "\"same_physical_device_import_tested\":false,\"composition_tested\":false,"
        "\"synchronization_tested\":false,\"foreign_queue_ownership_tested\":false,"
        "\"dma_buf_reservation_fences_tested\":false,\"hardware_performance_tested\":false}}\n",
        boolean(operational_error), result, boolean(any_eligible && !operational_error), boolean(any_current), status);
    return status;
}

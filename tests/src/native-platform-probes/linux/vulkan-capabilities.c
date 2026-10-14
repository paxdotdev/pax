#include <vulkan/vulkan.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Query actual export support before attempting GTK DMA-BUF interop. A Vulkan
 * loader or ICD filename alone does not identify a usable graphics adapter. */
int main(void) {
    VkApplicationInfo app = {.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO,
        .pApplicationName = "PAX-1038 Linux interop capabilities", .apiVersion = VK_API_VERSION_1_1};
    VkInstanceCreateInfo create = {.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO, .pApplicationInfo = &app};
    VkInstance instance;
    VkResult result = vkCreateInstance(&create, NULL, &instance);
    if (result != VK_SUCCESS) { fprintf(stderr, "vkCreateInstance=%d\n", result); return 1; }
    uint32_t count = 0;
    vkEnumeratePhysicalDevices(instance, &count, NULL);
    VkPhysicalDevice *devices = calloc(count, sizeof(*devices));
    vkEnumeratePhysicalDevices(instance, &count, devices);
    printf("physical_devices=%u\n", count);
    for (uint32_t d = 0; d < count; d++) {
        VkPhysicalDeviceProperties properties;
        vkGetPhysicalDeviceProperties(devices[d], &properties);
        printf("device=%s type=%u vendor=%04x device=%04x api=%u.%u.%u\n", properties.deviceName,
            properties.deviceType, properties.vendorID, properties.deviceID,
            VK_API_VERSION_MAJOR(properties.apiVersion), VK_API_VERSION_MINOR(properties.apiVersion),
            VK_API_VERSION_PATCH(properties.apiVersion));
        uint32_t extension_count = 0;
        vkEnumerateDeviceExtensionProperties(devices[d], NULL, &extension_count, NULL);
        VkExtensionProperties *extensions = calloc(extension_count, sizeof(*extensions));
        vkEnumerateDeviceExtensionProperties(devices[d], NULL, &extension_count, extensions);
        const char *wanted[] = {"VK_KHR_external_memory_fd", "VK_EXT_external_memory_dma_buf",
            "VK_EXT_image_drm_format_modifier", "VK_KHR_external_semaphore_fd", "VK_KHR_external_fence_fd"};
        for (size_t i = 0; i < sizeof(wanted)/sizeof(wanted[0]); i++) {
            int found = 0;
            for (uint32_t e = 0; e < extension_count; e++) found |= !strcmp(wanted[i], extensions[e].extensionName);
            printf("extension %s=%d\n", wanted[i], found);
        }
        VkFormat formats[] = {VK_FORMAT_R8G8B8A8_UNORM, VK_FORMAT_B8G8R8A8_UNORM};
        VkExternalMemoryHandleTypeFlagBits handles[] = {VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT,
            VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT};
        for (size_t f = 0; f < 2; f++) for (size_t h = 0; h < 2; h++) for (int linear = 0; linear < 2; linear++) {
            VkPhysicalDeviceExternalImageFormatInfo external = {
                .sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_EXTERNAL_IMAGE_FORMAT_INFO, .handleType = handles[h]};
            VkPhysicalDeviceImageFormatInfo2 info = {.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_IMAGE_FORMAT_INFO_2,
                .pNext = &external, .format = formats[f], .type = VK_IMAGE_TYPE_2D,
                .tiling = linear ? VK_IMAGE_TILING_LINEAR : VK_IMAGE_TILING_OPTIMAL,
                .usage = VK_IMAGE_USAGE_SAMPLED_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT};
            VkExternalImageFormatProperties external_properties = {.sType = VK_STRUCTURE_TYPE_EXTERNAL_IMAGE_FORMAT_PROPERTIES};
            VkImageFormatProperties2 output = {.sType = VK_STRUCTURE_TYPE_IMAGE_FORMAT_PROPERTIES_2, .pNext = &external_properties};
            result = vkGetPhysicalDeviceImageFormatProperties2(devices[d], &info, &output);
            printf("usage=sampled+transfer-dst format=%u handle=%u tiling=%s result=%d exportable=%d importable=%d dedicated=%d\n",
                formats[f], handles[h], linear ? "linear" : "optimal", result,
                !!(external_properties.externalMemoryProperties.externalMemoryFeatures & VK_EXTERNAL_MEMORY_FEATURE_EXPORTABLE_BIT),
                !!(external_properties.externalMemoryProperties.externalMemoryFeatures & VK_EXTERNAL_MEMORY_FEATURE_IMPORTABLE_BIT),
                !!(external_properties.externalMemoryProperties.externalMemoryFeatures & VK_EXTERNAL_MEMORY_FEATURE_DEDICATED_ONLY_BIT));
        }
        free(extensions);
    }
    free(devices);
    vkDestroyInstance(instance, NULL);
    return count ? 0 : 2;
}

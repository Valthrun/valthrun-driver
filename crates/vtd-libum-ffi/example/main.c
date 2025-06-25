#include <libum_ffi.h>
#include <stdio.h>
#include <Windows.h>

void check_status_exit(VtumStatus status, const char *message) {
    if (status == VTUM_STATUS_SUCCESS) {
        return;
    }

    printf("failed to %s. status: %x\n", message, status);
    exit(EXIT_FAILURE);
}

bool print_process(const ProcessInfo *info) {
    char image_base_name[16];
    image_base_name[15] = 0x00;
    memcpy(image_base_name, info->image_base_name, 15);

    printf(" - %u %s (directory table base = 0x%llX)\n", info->process_id, image_base_name, info->directory_table_base);
    return true;
}

bool print_process_module(const ProcessModuleInfo *info) {
    printf(" - %llx %s (size = %llx)\n", info->base_address, (const char *) info->base_dll_name, info->module_size);
    return true;
}

int main() {
    check_status_exit(vtum_library_initialize(), "failed to initialize lib");

    printf("VT library version: %s\n", vtum_library_version());

    InterfaceHandle *handle;
    check_status_exit(vtum_interface_create(&handle), "failed to create a new interface");

    VersionInfo version_info;
    vtum_interface_driver_version(handle, &version_info);

    DRIVER_FEATURE features;
    vtum_interface_driver_features(handle, &features);

    printf("Using driver %s version %d.%d.%d.\n",
           (const char *) version_info.application_name,
           version_info.version_major,
           version_info.version_minor,
           version_info.version_patch
    );

    DirectoryTableType directory_table;
    directory_table.tag = DIRECTORY_TABLE_TYPE_DEFAULT;

    DWORD current_process_id = GetCurrentProcessId();
    uint64_t target_value = 0xDEADBEEF;

    if (features.bits & DRIVER_FEATURE_MEMORY_READ.bits) {
        uint64_t read_buffer = 0x00;
        auto status = vtum_interface_memory_read(
            handle,
            current_process_id,
            &directory_table,
            (uint64_t) &target_value,
            (uint8_t *) &read_buffer, sizeof(read_buffer)
        );
        check_status_exit(status, "read dummy variable");
        printf("Read variable value: %llx\n", read_buffer);
    } else {
        printf("Driver does not support reading memory\n");
    }

    if (features.bits & DRIVER_FEATURE_MEMORY_WRITE.bits) {
        uint64_t new_value = 0xB00BB00B;
        auto status = vtum_interface_memory_write(
            handle,
            current_process_id,
            &directory_table,
            (uint64_t) &target_value,
            (uint8_t *) &new_value, sizeof(new_value)
        );
        check_status_exit(status, "write dummy variable");
        printf("Write variable value: %llx\n", target_value);
    } else {
        printf("Driver does not support writing memory\n");
    }

    if (features.bits & DRIVER_FEATURE_PROCESS_LIST.bits) {
        printf("Current process list:\n");
        check_status_exit(vtum_interface_process_list(handle, print_process), "iterate processes");
    } else {
        printf("Driver does not support iterating processes\n");
    }

    if (features.bits & DRIVER_FEATURE_PROCESS_MODULES.bits) {
        printf("Current processes modules:\n");
        check_status_exit(
            vtum_interface_process_module_list(handle, current_process_id, &directory_table, print_process_module),
            "iterate processes");
    } else {
        printf("Driver does not support iterating modules\n");
    }

    return 0;
}

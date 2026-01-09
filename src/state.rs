use std::ffi::CString;
use ash::{vk, Entry};
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

pub struct VulkanState {
    instance: ash::Instance,
}

impl VulkanState {
    pub fn new(entry: &Entry, window: &Window) -> Self {
        let app_name = CString::new("Vulkan Application").expect("CString");
        let engine_name = CString::new("Raw Dog").expect("CString");

        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name.as_c_str())
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(engine_name.as_c_str())
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::make_api_version(0, 1, 0, 0));

        let extension_names = ash_window::enumerate_required_extensions(
            window.display_handle().unwrap().as_raw(),
        )
        .expect("required extensions");

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(extension_names);

        let instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("vkCreateInstance")
        };

        Self { instance }
    }

    pub fn destroy(self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}



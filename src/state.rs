use ash::{Entry, vk};
use std::ffi::{CStr, CString};
use tracing::error;
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

const ENABLE_VALIDATION_LAYERS: bool = cfg!(debug_assertions);
const VALIDATION_LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

pub struct VulkanState {
    instance: ash::Instance,
}

impl VulkanState {
    pub fn new(entry: &Entry, window: &Window) -> Self {
        if ENABLE_VALIDATION_LAYERS && !Self::check_validation_layers(entry) {
            error!("Did not support all validation layers!");
        }

        let app_name = CString::new("WE VULKAN").expect("CString");
        let engine_name = CString::new("Raw Dog").expect("CString");

        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name.as_c_str())
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(engine_name.as_c_str())
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::make_api_version(0, 1, 0, 0));

        // The reason we can pass it the display handle is because the display
        // handle is actually an enum specifying wayland
        let extension_names =
            ash_window::enumerate_required_extensions(window.display_handle().unwrap().as_raw())
                .expect("required extensions");

        let layer_cstrings: Vec<CString> = if ENABLE_VALIDATION_LAYERS {
            VALIDATION_LAYERS
                .iter()
                .map(|s| CString::new(*s).unwrap())
                .collect()
        } else {
            Vec::new()
        };

        let layer_name_ptrs: Vec<*const i8> = layer_cstrings.iter().map(|s| s.as_ptr()).collect();

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(extension_names)
            .enabled_layer_names(&layer_name_ptrs);

        let instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("vkCreateInstance")
        };

        Self { instance }
    }

    // Ensure the validation (debugging) layers are supported
    fn check_validation_layers(entry: &Entry) -> bool {
        let available_layers = unsafe {
            entry
                .enumerate_instance_layer_properties()
                .expect("enumerate_instance_layer_properties")
        };

        for required_layer in VALIDATION_LAYERS {
            let mut found = false;

            for layer in &available_layers {
                let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) }
                    .to_str()
                    .unwrap_or("none utf8");

                if name == required_layer {
                    found = true;
                    break;
                }
            }

            if !found {
                error!("Validation layer {} is not supported", required_layer);
                return false;
            }
        }
        return true;
    }

    pub fn destroy(self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}

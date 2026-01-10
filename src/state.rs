use ash::{
    Entry, Instance,
    ext::debug_utils,
    vk::{self, PhysicalDevice},
};
use std::ffi::{CStr, CString, c_void};
use tracing::{error, info, warn};
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

const ENABLE_VALIDATION_LAYERS: bool = cfg!(debug_assertions);
const VALIDATION_LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

pub struct VulkanState {
    entry: Entry,
    instance: Instance,
    device: PhysicalDevice,

    debug_utils: Option<ash::ext::debug_utils::Instance>,
    debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
}

macro_rules! debug_messenger_create_info {
    () => {{
        ash::vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                ash::vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                ash::vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | ash::vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | ash::vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_callback))
            .user_data(std::ptr::null_mut())
    }};
}

extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    unsafe {
        let message = CStr::from_ptr(p_callback_data.as_ref().unwrap().p_message);
        let output = format!(
            "[VULKAN] {:?} {:?}: {}",
            message_severity,
            message_type,
            message.to_string_lossy()
        );
        match message_severity {
            s if s.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) => {
                error!(output);
            }
            s if s.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) => {
                warn!(output);
            }
            s if s.contains(vk::DebugUtilsMessageSeverityFlagsEXT::INFO) => {
                info!(output);
            }
            _ => {
                info!(output);
            }
        }

        vk::FALSE
    }
}

impl VulkanState {
    pub fn new(window: &Window) -> Self {
        let entry = Entry::linked();

        if ENABLE_VALIDATION_LAYERS {
            if !Self::check_validation_layers(&entry) {
                panic!("Did not support all validation layers!");
            }
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
        let extension_names = Self::get_required_extensions(window);

        let layer_cstrings: Vec<CString> = if ENABLE_VALIDATION_LAYERS {
            VALIDATION_LAYERS
                .iter()
                .map(|s| CString::new(*s).unwrap())
                .collect()
        } else {
            Vec::new()
        };

        let layer_name_ptrs: Vec<*const i8> = layer_cstrings.iter().map(|s| s.as_ptr()).collect();

        // Options for the instance
        let mut debug_create_info = debug_messenger_create_info!();
        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names)
            .enabled_layer_names(&layer_name_ptrs);

        if ENABLE_VALIDATION_LAYERS {
            // Ask the instance itself to also still emit debug events
            // from the callback, even when debug messenger isn't here yet
            create_info = create_info.push_next(&mut debug_create_info);
        }

        // Setting up default with the layers, options, extensions, etc. we want
        let instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("vkCreateInstance")
        };

        let device = Self::pick_device(&instance);
        let mut ret = Self {
            entry,
            instance,
            device,
            debug_utils: None,
            debug_messenger: None,
        };

        if ENABLE_VALIDATION_LAYERS {
            // THis creates a vulkan object that it keeps, we just
            // get a handle to it, which is why we need to do this again,
            // the previous debug_create_info is dropped because vulkan
            // doesn't own the object
            ret.setup_debug_messenger();
        }

        return ret;
    }

    fn pick_device(instance: &Instance) -> PhysicalDevice {
        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("Failed to enumerate physical devices")
        };

        for device in physical_devices {
            let properties = unsafe { instance.get_physical_device_properties(device) };
            let features = unsafe { instance.get_physical_device_features(device) };

            // Remove this in future, and do ranking instead
            let is_discrete = properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU;
            let has_geometry_shader = features.geometry_shader == vk::TRUE;
            if is_discrete && has_geometry_shader { return device; }
        }

        panic!("Failed to find a suitable physical device!");
    }

    // Load the debug messenger into vulkan / attach the callback
    fn setup_debug_messenger(&mut self) {
        let debug_utils_loader = debug_utils::Instance::new(&self.entry, &self.instance);

        let create_info = debug_messenger_create_info!();

        let messenger = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&create_info, None)
                .expect("create_debug_utils_messenger")
        };

        assert!(
            messenger != vk::DebugUtilsMessengerEXT::null(),
            "debug messenger is null"
        );

        self.debug_utils = Some(debug_utils_loader);
        self.debug_messenger = Some(messenger);
    }

    // Make a list of extensions we need
    fn get_required_extensions(window: &Window) -> Vec<*const i8> {
        let mut extension_names =
            ash_window::enumerate_required_extensions(window.display_handle().unwrap().as_raw())
                .expect("required extensions")
                .to_vec();

        if ENABLE_VALIDATION_LAYERS {
            extension_names.push(vk::EXT_DEBUG_UTILS_NAME.as_ptr());
        }

        return extension_names;
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
            if ENABLE_VALIDATION_LAYERS {
                self.debug_utils
                    .unwrap()
                    .destroy_debug_utils_messenger(self.debug_messenger.unwrap(), None);
            }

            self.instance.destroy_instance(None);
        }
    }
}

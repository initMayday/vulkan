use ash::{
    Device, Entry, Instance, ext, khr,
    vk::{self, PhysicalDevice, Queue, SurfaceKHR},
};
use std::{
    ffi::{CStr, CString, c_void},
    os::raw::c_char,
};
use tracing::{error, info, warn};
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

const ENABLE_VALIDATION_LAYERS: bool = cfg!(debug_assertions);
const VALIDATION_LAYERS: [&CStr; 1] = unsafe {
    [CStr::from_bytes_with_nul_unchecked(
        b"VK_LAYER_KHRONOS_validation\0",
    )]
};
// const VALIDATION_LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

const DEVICE_EXTENSIONS: [*const c_char; 1] = [ash::khr::swapchain::NAME.as_ptr()];

struct QueueFamilyIndices {
    pub graphics_family: Option<u32>,
    pub present_family: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        return self.graphics_family.is_some() && self.present_family.is_some();
    }
}

struct SwapChainSupportDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

pub struct VulkanState {
    entry: Entry,
    instance: Instance,
    physical_device: PhysicalDevice,
    logical_device: Device,

    graphics_queue: Queue,
    present_queue: Queue,

    surface: SurfaceKHR,
    surface_loader: khr::surface::Instance,

    debug_utils_loader: Option<ext::debug_utils::Instance>,
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

        // vk::True means abort program, vk::False, means continue, but just log
        // in future it may be better to abort on errors
        vk::FALSE
    }
}

impl VulkanState {
    pub fn new(window: &Window) -> Self {
        let entry = Entry::linked();
        let instance = Self::create_instance(&entry, window);
        let (surface, surface_loader) = Self::create_surface(&entry, &instance, window);
        let physical_device = Self::pick_physical_device(&instance, surface, &surface_loader);
        let (logical_device, graphics_queue, present_queue) =
            Self::create_logical_device(&instance, physical_device, surface, &surface_loader);

        let mut ret = Self {
            entry,
            instance,
            physical_device,
            logical_device,

            graphics_queue,
            present_queue,

            surface,
            surface_loader,

            debug_utils_loader: None,
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

    fn create_instance(entry: &Entry, window: &Window) -> Instance {
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

        let layer_name_ptrs: Vec<*const i8> = if ENABLE_VALIDATION_LAYERS {
            VALIDATION_LAYERS.iter().map(|s| s.as_ptr()).collect()
        } else {
            Vec::new()
        };

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

        return instance;
    }

    fn pick_physical_device(
        instance: &Instance,
        surface: SurfaceKHR,
        surface_loader: &khr::surface::Instance,
    ) -> PhysicalDevice {
        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("Failed to enumerate physical devices")
        };

        for device in physical_devices {
            if Self::is_physical_device_suitable(instance, device, surface, surface_loader) {
                return device;
            }
        }

        panic!("Failed to find a suitable physical device!");
    }

    fn is_physical_device_suitable(
        instance: &Instance,
        device: PhysicalDevice,
        surface: SurfaceKHR,
        surface_loader: &khr::surface::Instance,
    ) -> bool {
        let indices = Self::find_queue_families(instance, device, surface, surface_loader);
        if indices.is_complete() == false { return false }

        let extensions_supported: bool =
            Self::check_physical_device_extension_support(instance, device);
        if extensions_supported == false { return false; }

        let swap_chain_details = Self::query_swap_chain_support(device, surface, surface_loader);
        let swap_chain_adequate = !swap_chain_details.formats.is_empty() && !swap_chain_details.present_modes.is_empty();


        return swap_chain_adequate;
    }

    fn check_physical_device_extension_support(
        instance: &Instance,
        device: PhysicalDevice,
    ) -> bool {
        let available: Vec<vk::ExtensionProperties> = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .expect("Unable to enumerate device extensions")
        };

        for required in DEVICE_EXTENSIONS {
            let required = unsafe { CStr::from_ptr(required) };

            let found = available.iter().any(|extension| unsafe {
                CStr::from_ptr(extension.extension_name.as_ptr()) == required
            });
            if found == false {
                return false;
            }
        }

        return true;
    }

    fn find_queue_families(
        instance: &Instance,
        device: PhysicalDevice,
        surface: SurfaceKHR,
        surface_loader: &khr::surface::Instance,
    ) -> QueueFamilyIndices {
        let mut indices = QueueFamilyIndices {
            graphics_family: None,
            present_family: None,
        };

        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(device) };

        for (i, queue_family) in queue_families.iter().enumerate() {
            if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                indices.graphics_family = Some(i as u32);
            }

            let present_support = unsafe {
                surface_loader
                    .get_physical_device_surface_support(device, i as u32, surface)
                    .unwrap()
            };
            if present_support {
                indices.present_family = Some(i as u32);
            }

            if indices.is_complete() {
                break;
            }
        }

        return indices;
    }

    fn create_logical_device(
        instance: &Instance,
        physical_device: PhysicalDevice,
        surface: SurfaceKHR,
        surface_loader: &khr::surface::Instance,
    ) -> (Device, Queue, Queue) {
        let indicies =
            Self::find_queue_families(instance, physical_device, surface, surface_loader);

        let graphics_index = indicies.graphics_family.unwrap();
        let present_index = indicies.present_family.unwrap();

        let mut vec = vec![graphics_index, present_index];
        vec.dedup(); // The queue families are sometimes the same

        let queue_priorities = [1.0_f32];

        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = vec
            .iter()
            .copied()
            .map(|family_index| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(family_index)
                    .queue_priorities(&queue_priorities) // request 1 queue from this family
            })
            .collect();

        let device_features = vk::PhysicalDeviceFeatures::default();
        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&DEVICE_EXTENSIONS)
            .enabled_features(&device_features);

        let device = unsafe {
            instance
                .create_device(physical_device, &device_create_info, None)
                .expect("Failed to create logical device")
        };

        let graphics_queue = unsafe { device.get_device_queue(graphics_index, 0) };
        let present_queue = unsafe { device.get_device_queue(present_index, 0) };

        return (device, graphics_queue, present_queue);
    }

    fn query_swap_chain_support(
        device: PhysicalDevice,
        surface: SurfaceKHR,
        surface_loader: &khr::surface::Instance,
    ) -> SwapChainSupportDetails {
        let capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(device, surface)
                .expect("Could not get surface capabilites")
        };

        let formats = unsafe {
            surface_loader.get_physical_device_surface_formats(device, surface).expect("Could not get surface formats")
        };

        let present_modes = unsafe {
            surface_loader.get_physical_device_surface_present_modes(device, surface).expect("Could not get present modes")
        };

        SwapChainSupportDetails { capabilities, formats, present_modes }
    }

    // Returns the window surface, and the surface loader
    fn create_surface(
        entry: &Entry,
        instance: &Instance,
        window: &Window,
    ) -> (SurfaceKHR, khr::surface::Instance) {
        return (
            unsafe {
                ash_window::create_surface(
                    entry,
                    instance,
                    window.display_handle().unwrap().as_raw(),
                    window.window_handle().unwrap().as_raw(),
                    None,
                )
                .expect("Unable to create surface")
            },
            ash::khr::surface::Instance::new(entry, instance),
        );
    }

    // Load the debug messenger into vulkan / attach the callback
    fn setup_debug_messenger(&mut self) {
        let debug_utils_loader = ext::debug_utils::Instance::new(&self.entry, &self.instance);

        let create_info = debug_messenger_create_info!();

        let messenger = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&create_info, None)
                .expect("create_debug_utils_loader_messenger")
        };

        assert!(
            messenger != vk::DebugUtilsMessengerEXT::null(),
            "debug messenger is null"
        );

        self.debug_utils_loader = Some(debug_utils_loader);
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
                let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };

                if name == required_layer {
                    found = true;
                    break;
                }
            }

            if !found {
                error!(
                    "Validation layer {} is not supported",
                    required_layer.to_string_lossy()
                );
                return false;
            }
        }
        return true;
    }

    pub fn destroy(self) {
        unsafe {
            self.logical_device.destroy_device(None);

            if ENABLE_VALIDATION_LAYERS {
                self.debug_utils_loader
                    .unwrap()
                    .destroy_debug_utils_messenger(self.debug_messenger.unwrap(), None);
            }

            self.surface_loader.destroy_surface(self.surface, None);

            self.instance.destroy_instance(None);
        }
    }
}

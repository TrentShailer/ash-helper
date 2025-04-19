use std::{
    fs,
    io::{self, ErrorKind},
    path::Path,
};

/// A Vulkan layer to be initialized on application start.
pub struct VulkanLayer {
    /// The file name of the layer's manifest.
    pub manifest_file_name: &'static str,
    /// The binary data in the manifest.
    pub manifest: &'static [u8],

    /// The file name of the layer's binary.
    pub binary_file_name: &'static str,
    /// The binary.
    pub binary: &'static [u8],
}

impl VulkanLayer {
    /// Create a new Vulkan layer.
    pub fn new(
        manifest_file_name: &'static str,
        manifest: &'static [u8],
        binary_file_name: &'static str,
        binary: &'static [u8],
    ) -> Self {
        Self {
            manifest_file_name,
            manifest,
            binary_file_name,
            binary,
        }
    }

    /// Writes the layers to the directory specified then sets the `VK_LAYER_PATH` accordingly.
    ///
    /// # Safety
    /// * Reads and writes to the environment variable `VK_LAYER_PATH`.
    pub unsafe fn setup_layers(layers: &[Self], directory: &Path) -> io::Result<()> {
        // Validate directory and create if needed.
        {
            let directory_metadata = match directory.metadata() {
                Ok(metadata) => Some(metadata),
                Err(error) => {
                    if error.kind() == ErrorKind::NotFound {
                        None
                    } else {
                        return Err(error);
                    }
                }
            };

            match directory_metadata {
                Some(metadata) => {
                    if !metadata.is_dir() {
                        return Err(io::Error::new(
                            ErrorKind::NotADirectory,
                            format!("{directory:?} is not a directory"),
                        ));
                    }
                }

                None => fs::create_dir_all(directory)?,
            }
        }

        // For each layer, write to their respective file
        for layer in layers {
            let manifest_path = directory.join(layer.manifest_file_name);
            if !manifest_path.try_exists()? || fs::read(&manifest_path)? != layer.manifest {
                fs::write(manifest_path, layer.manifest)?;
                tracing::debug!("Wrote {}", layer.manifest_file_name);
            } else {
                tracing::debug!("Skipped {}", layer.manifest_file_name);
            }

            let binary_path = directory.join(layer.binary_file_name);
            if !binary_path.try_exists()? || fs::read(&binary_path)? != layer.binary {
                fs::write(binary_path, layer.binary)?;
                tracing::debug!("Wrote {}", layer.binary_file_name);
            } else {
                tracing::debug!("Skipped {}", layer.binary_file_name);
            }
        }

        // Add directory to path
        {
            let new_layer_path = directory.as_os_str();

            let vk_layer_path = match std::env::var_os("VK_LAYER_PATH") {
                Some(mut vk_layer_path) => {
                    vk_layer_path.push(";");
                    vk_layer_path.push(new_layer_path);
                    vk_layer_path
                }

                None => new_layer_path.to_owned(),
            };

            unsafe { std::env::set_var("VK_LAYER_PATH", vk_layer_path) };
        }

        Ok(())
    }
}

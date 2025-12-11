// Scene module for IntSar-3D

use crate::math::Transform;

/// Represents an object within the 3D scene.
#[derive(Debug, Clone)]
pub struct SceneObject {
    pub name: String,
    pub transform: Transform,
    // TODO: Add mesh, material, etc.
}

impl SceneObject {
    /// Creates a new scene object with a given name and transform.
    pub fn new(name: String, transform: Transform) -> Self {
        Self { name, transform }
    }
}

/// Represents the entire 3D scene.
#[derive(Debug, Default)]
pub struct Scene {
    pub objects: Vec<SceneObject>,
}

impl Scene {
    /// Creates a new, empty scene.
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    /// Adds an object to the scene.
    pub fn add_object(&mut self, object: SceneObject) {
        self.objects.push(object);
    }

    /// Gets a mutable reference to an object by name.
    pub fn get_object_mut(&mut self, name: &str) -> Option<&mut SceneObject> {
        self.objects.iter_mut().find(|obj| obj.name == name)
    }

    /// Gets an immutable reference to an object by name.
    pub fn get_object(&self, name: &str) -> Option<&SceneObject> {
        self.objects.iter().find(|obj| obj.name == name)
    }
}
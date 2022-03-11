use bevy_utils::tracing::warn;

use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    ptr::SemiSafeCell,
    query::Access,
    world::World,
};
use std::borrow::Cow;

/// An ECS system, typically converted from functions and closures whose arguments all implement
/// [`SystemParam`](crate::system::SystemParam).
///
/// **Note**: Only systems with `In = ()` and `Out = ()` can be added to a [`Schedule`](crate::schedule::Schedule).
/// When constructing a `Schedule`, use a [`SystemDescriptor`](crate::schedule::SystemDescriptor) to
/// specify when a system runs relative to others.
pub trait System: Send + Sync + 'static {
    /// The input to the system.
    type In;
    /// The output of the system.
    type Out;
    /// Returns the system's name.
    fn name(&self) -> Cow<'static, str>;
    /// Updates the archetype component [`Access`] of the system to account for `archetype`.
    fn new_archetype(&mut self, archetype: &Archetype);
    /// Returns the system's component [`Access`].
    fn component_access(&self) -> &Access<ComponentId>;
    /// Returns the system's current archetype component [`Access`].
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId>;
    /// Returns true if the system is [`Send`].
    fn is_send(&self) -> bool;
    /// Runs the system with the given `input` on `world`.
    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        // SAFETY: The world is exclusively borrowed.
        unsafe { self.run_unchecked(input, &SemiSafeCell::from_mut(world)) }
    }
    /// Runs the system with the given `input` on `world`.
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - The given world is the same one that was used to construct the system.
    /// - Systems do not concurrently access data in ways that violate Rust's
    /// rules for references.
    unsafe fn run_unchecked(&mut self, input: Self::In, world: &SemiSafeCell<World>) -> Self::Out;
    /// Applies deferred operations such as commands on the world.  
    fn apply_buffers(&mut self, world: &mut World);
    /// Initialize the system.
    fn initialize(&mut self, _world: &mut World);
    fn check_change_tick(&mut self, change_tick: u32);
    fn is_exclusive(&self) -> bool;
}

/// A convenient alias for a boxed [`System`] trait object.
pub type BoxedSystem<In = (), Out = ()> = Box<dyn System<In = In, Out = Out>>;

pub(crate) fn check_system_change_tick(
    last_change_tick: &mut u32,
    change_tick: u32,
    system_name: &str,
) {
    let tick_delta = change_tick.wrapping_sub(*last_change_tick);
    const MAX_DELTA: u32 = (u32::MAX / 4) * 3;
    // Clamp to max delta
    if tick_delta > MAX_DELTA {
        warn!(
            "Too many intervening systems have run since the last time System '{}' was last run; it may fail to detect changes.",
            system_name
        );
        *last_change_tick = change_tick.wrapping_sub(MAX_DELTA);
    }
}

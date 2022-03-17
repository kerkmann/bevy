use crate::{
    archetype::{Archetype, ArchetypeComponentId, ArchetypeGeneration, ArchetypeId},
    component::ComponentId,
    ptr::SemiSafeCell,
    query::{Access, FilteredAccessSet},
    system::{
        check_system_change_tick, ReadOnlySystemParamFetch, System, SystemParam, SystemParamFetch,
        SystemParamState,
    },
    world::{World, WorldId},
};
use bevy_ecs_macros::all_tuples;
use std::{borrow::Cow, marker::PhantomData};

/// The metadata of a [`System`].
pub struct SystemMeta {
    pub(crate) name: Cow<'static, str>,
    pub(crate) component_access_set: FilteredAccessSet<ComponentId>,
    pub(crate) archetype_component_access: Access<ArchetypeComponentId>,
    pub(crate) last_change_tick: u32,
    // NOTE: This field must be kept private. Making a `SystemMeta` non-`Send` is irreversible to
    // prevent multiple system params from toggling it.
    is_send: bool,
    // NOTE: This field was a tempoary measure to remove `.exclusive_system()` without disturbing
    // the rest of the existing API. See #4166.
    is_exclusive: bool,
}

impl SystemMeta {
    fn new<T>() -> Self {
        Self {
            name: std::any::type_name::<T>().into(),
            archetype_component_access: Access::default(),
            component_access_set: FilteredAccessSet::default(),
            last_change_tick: 0,
            is_send: true,
            is_exclusive: false,
        }
    }

    /// Returns true if the system is [`Send`].
    #[inline]
    pub fn is_send(&self) -> bool {
        self.is_send
    }

    /// Sets the system to be not [`Send`].
    ///
    /// This is irreversible.
    #[inline]
    pub fn set_non_send(&mut self) {
        self.is_send = false;
    }

    #[inline]
    pub(crate) fn is_exclusive(&self) -> bool {
        self.is_exclusive
    }

    #[inline]
    pub(crate) fn set_exclusive(&mut self) {
        self.is_exclusive = true;
    }
}

// TODO: Actually use this in FunctionSystem. We should probably only do this once Systems are constructed using a World reference
// (to avoid the need for unwrapping to retrieve SystemMeta)
/// Holds on to persistent state required to drive [`SystemParam`] for a [`System`].
///
/// This is a very powerful and convenient tool for working with exclusive world access,
/// allowing you to fetch data from the [`World`] as if you were running a [`System`].
///
/// Borrow-checking is handled for you, allowing you to mutably access multiple compatible system parameters at once,
/// and arbitrary system parameters (like [`EventWriter`](crate::event::EventWriter)) can be conveniently fetched.
///
/// For an alternative approach to split mutable access to the world, see [`World::resource_scope`].
///
/// # Warning
///
/// [`SystemState`] values created can be cached to improve performance,
/// and *must* be cached and reused in order for system parameters that rely on local state to work correctly.
/// These include:
/// - [`Added`](crate::query::Added) and [`Changed`](crate::query::Changed) query filters
/// - [`Local`](crate::system::Local) variables that hold state
/// - [`EventReader`](crate::event::EventReader) system parameters, which rely on a [`Local`](crate::system::Local) to track which events have been seen
///
/// # Example
///
/// Basic usage:
/// ```rust
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::{system::SystemState};
/// use bevy_ecs::event::Events;
///
/// struct MyEvent;
/// struct MyResource(u32);
///
/// #[derive(Component)]
/// struct MyComponent;
///
/// // Work directly on the `World`
/// let mut world = World::new();
/// world.init_resource::<Events<MyEvent>>();
///
/// // Construct a `SystemState` struct, passing in a tuple of `SystemParam`
/// // as if you were writing an ordinary system.
/// let mut system_state: SystemState<(
///     EventWriter<MyEvent>,
///     Option<ResMut<MyResource>>,
///     Query<&MyComponent>,
///     )> = SystemState::new(&mut world);
///
/// // Use system_state.get_mut(&mut world) and unpack your system parameters into variables!
/// // system_state.get(&world) is available if your params are read-only.
/// let (event_writer, maybe_resource, query) = system_state.get_mut(&mut world);
/// ```
/// Caching:
/// ```rust
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::{system::SystemState};
/// use bevy_ecs::event::Events;
///
/// struct MyEvent;
/// struct CachedSystemState<'w, 's>{
///    event_state: SystemState<EventReader<'w, 's, MyEvent>>
/// }
///
/// // Create and store a system state once
/// let mut world = World::new();
/// world.init_resource::<Events<MyEvent>>();
/// let initial_state: SystemState<EventReader<MyEvent>>  = SystemState::new(&mut world);
///
/// // The system state is cached in a resource
/// world.insert_resource(CachedSystemState{event_state: initial_state});
///
/// // Later, fetch the cached system state, saving on overhead
/// world.resource_scope(|world, mut cached_state: Mut<CachedSystemState>| {
///     let mut event_reader = cached_state.event_state.get_mut(world);
///
///     for events in event_reader.iter(){
///         println!("Hello World!");
///     };
/// });
/// ```
pub struct SystemState<Param: SystemParam> {
    meta: SystemMeta,
    param_state: <Param as SystemParam>::Fetch,
    world_id: WorldId,
    archetype_generation: ArchetypeGeneration,
}

impl<Param: SystemParam> SystemState<Param> {
    pub fn new(world: &mut World) -> Self {
        let mut meta = SystemMeta::new::<Param>();
        let param_state = <Param::Fetch as SystemParamState>::init(world, &mut meta);
        Self {
            meta,
            param_state,
            world_id: world.id(),
            archetype_generation: ArchetypeGeneration::initial(),
        }
    }

    #[inline]
    pub fn meta(&self) -> &SystemMeta {
        &self.meta
    }

    /// Retrieves the [`SystemParam`] values (must all be read-only) from the given [`World`].
    ///
    /// This method also ensures the cached access is up-to-date before retrieving the data.
    #[inline]
    pub fn get<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> <Param::Fetch as SystemParamFetch<'w, 's>>::Item
    where
        Param::Fetch: ReadOnlySystemParamFetch,
    {
        self.validate_world_and_update_archetypes(world);
        // SAFETY: The params cannot request mutable access and world is the same one used to construct this state.
        unsafe { self.get_unchecked(&SemiSafeCell::from_ref(world)) }
    }

    /// Retrieves the [`SystemParam`] values from the given [`World`].
    ///
    /// This method also ensures the cached access is up-to-date before retrieving the data.
    #[inline]
    pub fn get_mut<'w, 's>(
        &'s mut self,
        world: &'w mut World,
    ) -> <Param::Fetch as SystemParamFetch<'w, 's>>::Item {
        self.validate_world_and_update_archetypes(world);
        // SAFETY: The world is exclusively borrowed and the same one used to construct this state.
        unsafe { self.get_unchecked(&SemiSafeCell::from_mut(world)) }
    }

    /// Applies any state queued by [`SystemParam`] values to the given [`World`].
    ///
    /// As an example, this will apply any commands queued using [`Commands`](`super::Commands`).
    pub fn apply(&mut self, world: &mut World) {
        self.param_state.apply(world);
    }

    #[inline]
    pub fn matches_world(&self, world: &World) -> bool {
        self.world_id == world.id()
    }

    fn validate_world_and_update_archetypes(&mut self, world: &World) {
        assert!(self.matches_world(world), "Encountered a mismatched World. A SystemState cannot be used with Worlds other than the one it was created with.");
        let archetypes = world.archetypes();
        let new_generation = archetypes.generation();
        let old_generation = std::mem::replace(&mut self.archetype_generation, new_generation);
        let archetype_index_range = old_generation.value()..new_generation.value();

        for archetype_index in archetype_index_range {
            self.param_state.new_archetype(
                &archetypes[ArchetypeId::new(archetype_index)],
                &mut self.meta,
            );
        }
    }

    /// Retrieves the [`SystemParam`] values from the given [`World`].
    ///
    /// This method does _not_ update the system state's cached access before retrieving the data.
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - The given world is the same world used to construct the system state.
    /// - There are no active references that conflict with the system state's access. Mutable access must be unique.
    #[inline]
    pub unsafe fn get_unchecked<'w, 's>(
        &'s mut self,
        world: &SemiSafeCell<'w, World>,
    ) -> <Param::Fetch as SystemParamFetch<'w, 's>>::Item {
        let change_tick = world.as_ref().increment_change_tick();
        let param = <Param::Fetch as SystemParamFetch>::get_param(
            &mut self.param_state,
            &self.meta,
            world,
            change_tick,
        );
        self.meta.last_change_tick = change_tick;
        param
    }
}

/// Conversion trait to turn something into a [`System`].
///
/// Use this to get a system from a function. Also note that every system implements this trait as
/// well.
///
/// # Examples
///
/// ```
/// use bevy_ecs::system::IntoSystem;
/// use bevy_ecs::system::Res;
///
/// fn my_system_function(an_usize_resource: Res<usize>) {}
///
/// let system = IntoSystem::system(my_system_function);
/// ```
// This trait has to be generic because we have potentially overlapping impls, in particular
// because Rust thinks a type could impl multiple different `FnMut` combinations
// even though none can currently
pub trait IntoSystem<In, Out, Params>: Sized {
    type System: System<In = In, Out = Out>;
    /// Turns this value into its corresponding [`System`].
    ///
    /// Use of this method was formerly required whenever adding a `system` to an `App`.
    /// or other cases where a system is required.
    /// However, since [#2398](https://github.com/bevyengine/bevy/pull/2398),
    /// this is no longer required.
    ///
    /// In future, this method will be removed.
    ///
    /// One use of this method is to assert that a given function is a valid system.
    /// For this case, use [`bevy_ecs::system::assert_is_system`] instead.
    ///
    /// [`bevy_ecs::system::assert_is_system`]: [`crate::system::assert_is_system`]:
    #[deprecated(
        since = "0.7.0",
        note = "`.system()` is no longer needed, as methods which accept systems will convert functions into a system automatically"
    )]
    fn system(self) -> Self::System {
        IntoSystem::into_system(self)
    }
    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;
}

pub struct AlreadyWasSystem;

// Systems implicitly implement IntoSystem
impl<In, Out, Sys: System<In = In, Out = Out>> IntoSystem<In, Out, AlreadyWasSystem> for Sys {
    type System = Sys;
    fn into_system(this: Self) -> Sys {
        this
    }
}

/// Wrapper type to mark a [`SystemParam`] as an input.
///
/// [`System`]s may take an optional input which they require to be passed to them when they
/// are being [`run`](System::run). For [`FunctionSystems`](FunctionSystem) the input may be marked
/// with this `In` type, but only the first param of a function may be tagged as an input. This also
/// means a system can only have one or zero input paramaters.
///
/// # Examples
///
/// Here is a simple example of a system that takes a [`usize`] returning the square of it.
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// fn main() {
///     let mut square_system = IntoSystem::into_system(square);
///
///     let mut world = World::default();
///     square_system.initialize(&mut world);
///     assert_eq!(square_system.run(12, &mut world), 144);
/// }
///
/// fn square(In(input): In<usize>) -> usize {
///     input * input
/// }
/// ```
pub struct In<T>(pub T);
pub struct InputMarker;

/// The [`System`]-type of functions and closures.
///
/// Constructed by calling [`IntoSystem::into_system`] with a function or closure whose arguments all implement
/// [`SystemParam`].
///
/// If the function's first argument is [`In<T>`], `T` becomes the system's [`In`](crate::system::System::In) type,
/// `()` otherwise.
/// The function's return type becomes the system's [`Out`](crate::system::System::Out) type.
pub struct FunctionSystem<In, Out, Param, Marker, F>
where
    Param: SystemParam,
{
    func: F,
    param_state: Option<Param::Fetch>,
    system_meta: SystemMeta,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
    #[allow(clippy::type_complexity)]
    marker: PhantomData<fn() -> (In, Out, Marker)>,
}

pub struct IsFunctionSystem;

impl<In, Out, Param, Marker, F> IntoSystem<In, Out, (IsFunctionSystem, Param, Marker)> for F
where
    In: 'static,
    Out: 'static,
    Param: SystemParam + 'static,
    Marker: 'static,
    F: SystemParamFunction<In, Out, Param, Marker> + Send + Sync + 'static,
{
    type System = FunctionSystem<In, Out, Param, Marker, F>;
    fn into_system(func: Self) -> Self::System {
        FunctionSystem {
            func,
            param_state: None,
            system_meta: SystemMeta::new::<F>(),
            marker: PhantomData,
        }
    }
}

impl<In, Out, Param, Marker, F> System for FunctionSystem<In, Out, Param, Marker, F>
where
    In: 'static,
    Out: 'static,
    Param: SystemParam + 'static,
    Marker: 'static,
    F: SystemParamFunction<In, Out, Param, Marker> + Send + Sync + 'static,
{
    type In = In;
    type Out = Out;

    #[inline]
    fn name(&self) -> Cow<'static, str> {
        self.system_meta.name.clone()
    }

    #[inline]
    fn new_archetype(&mut self, archetype: &Archetype) {
        let param_state = self.param_state.as_mut().unwrap();
        param_state.new_archetype(archetype, &mut self.system_meta);
    }

    #[inline]
    fn component_access(&self) -> &Access<ComponentId> {
        self.system_meta.component_access_set.combined_access()
    }

    #[inline]
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.system_meta.archetype_component_access
    }

    #[inline]
    fn is_send(&self) -> bool {
        self.system_meta.is_send
    }

    #[inline]
    unsafe fn run_unchecked(&mut self, input: Self::In, world: &SemiSafeCell<World>) -> Self::Out {
        let change_tick = world.as_ref().increment_change_tick();
        let out = self.func.run(
            input,
            self.param_state.as_mut().unwrap(),
            &self.system_meta,
            world,
            change_tick,
        );
        self.system_meta.last_change_tick = change_tick;
        out
    }

    #[inline]
    fn apply_buffers(&mut self, world: &mut World) {
        let param_state = self.param_state.as_mut().unwrap();
        param_state.apply(world);
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        self.param_state = Some(<Param::Fetch as SystemParamState>::init(
            world,
            &mut self.system_meta,
        ));
    }

    #[inline]
    fn check_change_tick(&mut self, change_tick: u32) {
        check_system_change_tick(
            &mut self.system_meta.last_change_tick,
            change_tick,
            self.system_meta.name.as_ref(),
        );
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        self.system_meta.is_exclusive()
    }
}

/// Trait implemented for all functions that can implement [`System`].
//
// This trait requires the generic `Params` because, as far as Rust knows, a type could have
// more than one impl of `FnMut`, even though functions and closures don't.
pub trait SystemParamFunction<In, Out, Params: SystemParam, Marker>: Send + Sync + 'static {
    /// # Safety
    ///
    /// Caller must ensure:
    /// - That `state` was constructed from the given `world`.
    /// - There are no active references that conflict with `state`'s access. Mutable access must be unique.
    unsafe fn run(
        &mut self,
        input: In,
        state: &mut Params::Fetch,
        system_meta: &SystemMeta,
        world: &SemiSafeCell<World>,
        change_tick: u32,
    ) -> Out;
}

macro_rules! impl_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<Out, F, $($param),*> SystemParamFunction<(), Out, ($($param,)*), ()> for F
        where
            F: Send + Sync + 'static,
            $($param: SystemParam,)*
            Out: 'static,
            for <'a> &'a mut F:
                FnMut($($param,)*) -> Out +
                FnMut($(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item,)*) -> Out,
        {
            #[inline]
            unsafe fn run(
                &mut self,
                _input: (),
                state: &mut <($($param,)*) as SystemParam>::Fetch,
                system_meta: &SystemMeta,
                world: &SemiSafeCell<World>,
                change_tick: u32,
            ) -> Out {
                // Yes, this is strange, but rustc fails to compile this impl
                // without using this function.
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Out, $($param,)*>(
                    mut f: impl FnMut($($param,)*) -> Out,
                    $($param: $param,)*
                ) -> Out {
                    f($($param,)*)
                }
                let ($($param,)*) = <<($($param,)*) as SystemParam>::Fetch as SystemParamFetch>::get_param(
                    state,
                    system_meta,
                    world,
                    change_tick
                );
                call_inner(self, $($param,)*)
            }
        }

        #[allow(non_snake_case)]
        impl<Input, Out, F, $($param),*> SystemParamFunction<Input, Out, ($($param,)*), InputMarker> for F
        where
            F: Send + Sync + 'static,
            $($param: SystemParam,)*
            Out: 'static,
            for <'a> &'a mut F:
                FnMut(In<Input>, $($param,)*) -> Out +
                FnMut(In<Input>, $(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item,)*) -> Out,
        {
            #[inline]
            unsafe fn run(
                &mut self,
                input: Input,
                state: &mut <($($param,)*) as SystemParam>::Fetch,
                system_meta: &SystemMeta,
                world: &SemiSafeCell<World>,
                change_tick: u32,
            ) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Input, Out, $($param,)*>(
                    mut f: impl FnMut(In<Input>, $($param,)*) -> Out,
                    input: In<Input>,
                    $($param: $param,)*
                ) -> Out {
                    f(input, $($param,)*)
                }
                let ($($param,)*) = <<($($param,)*) as SystemParam>::Fetch as SystemParamFetch>::get_param(
                    state,
                    system_meta,
                    world,
                    change_tick
                );
                call_inner(self, In(input), $($param,)*)
            }
        }
    };
}

all_tuples!(impl_system_function, 0, 16, P);

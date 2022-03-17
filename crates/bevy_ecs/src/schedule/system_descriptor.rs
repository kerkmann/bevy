use crate::{
    schedule::{
        AmbiguitySetLabel, BoxedAmbiguitySetLabel, BoxedSystemLabel, IntoRunCriteria,
        RunCriteriaDescriptorOrLabel, SystemLabel,
    },
    system::{BoxedSystem, IntoSystem},
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum InsertionPoint {
    AtStart,
    BeforeCommands,
    AtEnd,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SystemType {
    Parallel,
    Exclusive(InsertionPoint),
}

pub struct SystemLabelMarker;

/// Encapsulates a system and information on when it should run within a [`SystemStage`](crate::schedule::SystemStage).
///
/// A system can be inserted into one of four phases within a stage:
/// - **At start**: Runs systems sequentially, before the systems in **parallel**.
/// - **Parallel**: Runs systems concurrently, when possible. (Systems that borrow [`&mut World`](crate::world::World) cannot be inserted here).
/// - **Before commands**, Runs systems sequentially, before the commands queued by systems in **parallel** are applied.
/// - **At end**, Runs systems sequentially, after the commands queued by systems in **parallel** have been applied.
///
/// A system may have zero or many labels. Systems can specify their order relative to other systems
/// *in the same phase* using the [`.before()`](SystemDescriptor::before) and [`.after()`](SystemDescriptor::after) methods.
///
/// # Example
/// ```
/// # use bevy_ecs::prelude::*;
/// # fn do_something() {}
/// # fn do_the_other_thing() {}
/// # fn do_something_else() {}
/// #[derive(SystemLabel, Debug, Clone, PartialEq, Eq, Hash)]
/// struct Something;
///
/// SystemStage::parallel()
///     .with_system(do_something.label(Something))
///     .with_system(do_the_other_thing.after(Something))
///     .with_system(do_something_else.at_end());
/// ```
pub struct SystemDescriptor {
    pub(crate) system: BoxedSystem<(), ()>,
    pub(crate) run_criteria: Option<RunCriteriaDescriptorOrLabel>,
    pub(crate) labels: Vec<BoxedSystemLabel>,
    pub(crate) before: Vec<BoxedSystemLabel>,
    pub(crate) after: Vec<BoxedSystemLabel>,
    pub(crate) ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
    pub(crate) system_type: SystemType,
}

fn new_descriptor(system: BoxedSystem<(), ()>) -> SystemDescriptor {
    SystemDescriptor {
        system,
        run_criteria: None,
        labels: Vec::new(),
        before: Vec::new(),
        after: Vec::new(),
        ambiguity_sets: Vec::new(),
        system_type: SystemType::Parallel,
    }
}

impl SystemDescriptor {
    pub fn with_run_criteria<Marker>(
        mut self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        self.run_criteria = Some(run_criteria.into());
        self
    }

    pub fn label(mut self, label: impl SystemLabel) -> SystemDescriptor {
        self.labels.push(Box::new(label));
        self
    }

    pub fn before(mut self, label: impl SystemLabel) -> SystemDescriptor {
        self.before.push(Box::new(label));
        self
    }

    pub fn after(mut self, label: impl SystemLabel) -> SystemDescriptor {
        self.after.push(Box::new(label));
        self
    }

    pub fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> SystemDescriptor {
        self.ambiguity_sets.push(Box::new(set));
        self
    }

    pub fn at_start(mut self) -> SystemDescriptor {
        self.system_type = SystemType::Exclusive(InsertionPoint::AtStart);
        self
    }

    pub fn before_commands(mut self) -> SystemDescriptor {
        self.system_type = SystemType::Exclusive(InsertionPoint::BeforeCommands);
        self
    }

    pub fn at_end(mut self) -> SystemDescriptor {
        self.system_type = SystemType::Exclusive(InsertionPoint::AtEnd);
        self
    }
}

pub trait IntoSystemDescriptor<Params> {
    fn into_descriptor(self) -> SystemDescriptor;

    /// Assigns a run criteria to the system.
    /// This can be a new descriptor or a label of a run criteria defined elsewhere.
    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor;

    /// Assigns a label to the system; there can be more than one, and it doesn't have to be unique.
    fn label(self, label: impl SystemLabel) -> SystemDescriptor;

    /// Specifies that the system should run before systems with the given label.
    fn before(self, label: impl SystemLabel) -> SystemDescriptor;

    /// Specifies that the system should run after systems with the given label.
    fn after(self, label: impl SystemLabel) -> SystemDescriptor;

    /// Specifies that the system is exempt from execution order ambiguity detection
    /// with other systems in this set.
    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> SystemDescriptor;

    // /// This method was formerly required to add systems with `&mut World` arguments to an `App`.
    // ///
    // /// However, as of [#4166](https://github.com/bevyengine/bevy/pull/4166),
    // /// this is no longer required.
    // ///
    // /// In the future, this method will be removed.
    // #[deprecated(
    //     since = "0.7.0",
    //     note = "`.exclusive_system()` is no longer needed, bevy can convert these functions automatically"
    // )]
    fn exclusive_system(self) -> SystemDescriptor;

    /// Specifies that the system should run with other exclusive systems at the start of stage.
    fn at_start(self) -> SystemDescriptor;

    /// Specifies that the system should run with other exclusive systems after the parallel
    /// systems and before command buffer application.
    fn before_commands(self) -> SystemDescriptor;

    /// Specifies that the system should run with other exclusive systems at the end of stage.
    fn at_end(self) -> SystemDescriptor;
}

impl<S, Params> IntoSystemDescriptor<Params> for S
where
    S: IntoSystem<(), (), Params>,
{
    fn into_descriptor(self) -> SystemDescriptor {
        new_descriptor(Box::new(IntoSystem::into_system(self)))
    }

    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        new_descriptor(Box::new(IntoSystem::into_system(self))).with_run_criteria(run_criteria)
    }

    fn label(self, label: impl SystemLabel) -> SystemDescriptor {
        new_descriptor(Box::new(IntoSystem::into_system(self))).label(label)
    }

    fn before(self, label: impl SystemLabel) -> SystemDescriptor {
        new_descriptor(Box::new(IntoSystem::into_system(self))).before(label)
    }

    fn after(self, label: impl SystemLabel) -> SystemDescriptor {
        new_descriptor(Box::new(IntoSystem::into_system(self))).after(label)
    }

    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> SystemDescriptor {
        new_descriptor(Box::new(IntoSystem::into_system(self))).in_ambiguity_set(set)
    }

    fn exclusive_system(self) -> SystemDescriptor {
        new_descriptor(Box::new(IntoSystem::into_system(self))).at_start()
    }

    fn at_start(self) -> SystemDescriptor {
        new_descriptor(Box::new(IntoSystem::into_system(self))).at_start()
    }

    fn before_commands(self) -> SystemDescriptor {
        new_descriptor(Box::new(IntoSystem::into_system(self))).before_commands()
    }

    fn at_end(self) -> SystemDescriptor {
        new_descriptor(Box::new(IntoSystem::into_system(self))).at_end()
    }
}

impl IntoSystemDescriptor<()> for BoxedSystem<(), ()> {
    fn into_descriptor(self) -> SystemDescriptor {
        new_descriptor(self)
    }

    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        new_descriptor(self).with_run_criteria(run_criteria)
    }

    fn label(self, label: impl SystemLabel) -> SystemDescriptor {
        new_descriptor(self).label(label)
    }

    fn before(self, label: impl SystemLabel) -> SystemDescriptor {
        new_descriptor(self).before(label)
    }

    fn after(self, label: impl SystemLabel) -> SystemDescriptor {
        new_descriptor(self).after(label)
    }

    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> SystemDescriptor {
        new_descriptor(self).in_ambiguity_set(set)
    }

    fn exclusive_system(self) -> SystemDescriptor {
        new_descriptor(self).at_start()
    }

    fn at_start(self) -> SystemDescriptor {
        new_descriptor(self).at_start()
    }

    fn before_commands(self) -> SystemDescriptor {
        new_descriptor(self).before_commands()
    }

    fn at_end(self) -> SystemDescriptor {
        new_descriptor(self).at_end()
    }
}

impl IntoSystemDescriptor<()> for SystemDescriptor {
    fn into_descriptor(self) -> SystemDescriptor {
        self
    }

    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        self.with_run_criteria(run_criteria)
    }

    fn label(self, label: impl SystemLabel) -> SystemDescriptor {
        self.label(label)
    }

    fn before(self, label: impl SystemLabel) -> SystemDescriptor {
        self.before(label)
    }

    fn after(self, label: impl SystemLabel) -> SystemDescriptor {
        self.after(label)
    }

    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> SystemDescriptor {
        self.in_ambiguity_set(set)
    }

    fn exclusive_system(self) -> SystemDescriptor {
        self.at_start()
    }

    fn at_start(self) -> SystemDescriptor {
        self.at_start()
    }

    fn before_commands(self) -> SystemDescriptor {
        self.before_commands()
    }

    fn at_end(self) -> SystemDescriptor {
        self.at_end()
    }
}

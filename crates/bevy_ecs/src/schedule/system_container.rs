use crate::{
    component::ComponentId,
    query::Access,
    schedule::{
        BoxedAmbiguitySetLabel, BoxedRunCriteriaLabel, BoxedSystemLabel, GraphNode,
        SystemDescriptor,
    },
    system::{BoxedSystem, System},
};
use std::borrow::Cow;

/// System metadata like its name, labels, order requirements and component access.
pub trait SystemContainer: GraphNode<Label = BoxedSystemLabel> {
    #[doc(hidden)]
    fn dependencies(&self) -> &[usize];
    #[doc(hidden)]
    fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = usize>);
    #[doc(hidden)]
    fn run_criteria(&self) -> Option<usize>;
    #[doc(hidden)]
    fn set_run_criteria(&mut self, index: usize);
    fn run_criteria_label(&self) -> Option<&BoxedRunCriteriaLabel>;
    fn ambiguity_sets(&self) -> &[BoxedAmbiguitySetLabel];
    fn component_access(&self) -> Option<&Access<ComponentId>>;
}

pub struct FunctionSystemContainer {
    system: BoxedSystem,
    pub(crate) run_criteria_index: Option<usize>,
    pub(crate) run_criteria_label: Option<BoxedRunCriteriaLabel>,
    pub(crate) should_run: bool,
    dependencies: Vec<usize>,
    labels: Vec<BoxedSystemLabel>,
    before: Vec<BoxedSystemLabel>,
    after: Vec<BoxedSystemLabel>,
    ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
}

unsafe impl Send for FunctionSystemContainer {}
unsafe impl Sync for FunctionSystemContainer {}

impl FunctionSystemContainer {
    pub(crate) fn from_descriptor(descriptor: SystemDescriptor) -> Self {
        FunctionSystemContainer {
            system: descriptor.system,
            should_run: false,
            run_criteria_index: None,
            run_criteria_label: None,
            dependencies: Vec::new(),
            labels: descriptor.labels,
            before: descriptor.before,
            after: descriptor.after,
            ambiguity_sets: descriptor.ambiguity_sets,
        }
    }

    pub fn name(&self) -> Cow<'static, str> {
        GraphNode::name(self)
    }

    pub fn system(&self) -> &dyn System<In = (), Out = ()> {
        &*self.system
    }

    pub fn system_mut(&mut self) -> &mut dyn System<In = (), Out = ()> {
        &mut *self.system
    }

    pub fn should_run(&self) -> bool {
        self.should_run
    }

    pub fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }
}

impl GraphNode for FunctionSystemContainer {
    type Label = BoxedSystemLabel;

    fn name(&self) -> Cow<'static, str> {
        self.system().name()
    }

    fn labels(&self) -> &[BoxedSystemLabel] {
        &self.labels
    }

    fn before(&self) -> &[BoxedSystemLabel] {
        &self.before
    }

    fn after(&self) -> &[BoxedSystemLabel] {
        &self.after
    }
}

impl SystemContainer for FunctionSystemContainer {
    fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }

    fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = usize>) {
        self.dependencies.clear();
        self.dependencies.extend(dependencies);
    }

    fn run_criteria(&self) -> Option<usize> {
        self.run_criteria_index
    }

    fn set_run_criteria(&mut self, index: usize) {
        self.run_criteria_index = Some(index);
    }

    fn run_criteria_label(&self) -> Option<&BoxedRunCriteriaLabel> {
        self.run_criteria_label.as_ref()
    }

    fn ambiguity_sets(&self) -> &[BoxedAmbiguitySetLabel] {
        &self.ambiguity_sets
    }

    fn component_access(&self) -> Option<&Access<ComponentId>> {
        Some(self.system().component_access())
    }
}

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Result};
use phobos::domain::ExecutionDomain;
use phobos::query_pool::{PipelineStatisticsQuery, QueryPool, QueryPoolCreateInfo, TimestampQuery};
use phobos::wsi::frame::FRAMES_IN_FLIGHT;
use phobos::{vk, IncompleteCommandBuffer, PipelineStage};

use crate::gfx::SharedContext;
use crate::util::safe_error::SafeUnwrap;

#[derive(Debug, Default, Hash, Eq, PartialEq, Copy, Clone)]
struct SectionQuery {
    start_query: u32,
    end_query: u32,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct RendererStatistics {
    #[derivative(Debug = "ignore")]
    statistics: QueryPool<PipelineStatisticsQuery>,
    #[derivative(Debug = "ignore")]
    timings: QueryPool<TimestampQuery>,
    sections: HashMap<String, SectionQuery>,
    timing_results: HashMap<String, Duration>,
    interval: u32,
    frames_until_measure: u32,
}

impl RendererStatistics {
    pub fn new(ctx: SharedContext, section_capacity: u32, measure_interval: u32) -> Result<Self> {
        let timings = QueryPool::new(
            ctx.device.clone(),
            QueryPoolCreateInfo {
                count: section_capacity * 2,
                statistic_flags: None,
            },
        )?;

        let statistics = QueryPool::new(
            ctx.device.clone(),
            QueryPoolCreateInfo {
                count: section_capacity,
                // etc
                statistic_flags: Some(vk::QueryPipelineStatisticFlags::FRAGMENT_SHADER_INVOCATIONS),
            },
        )?;

        Ok(Self {
            statistics,
            timings,
            sections: Default::default(),
            timing_results: Default::default(),
            interval: measure_interval,
            frames_until_measure: measure_interval + 1,
        })
    }

    pub fn begin_section<'q, D: ExecutionDomain>(
        &mut self,
        cmd: IncompleteCommandBuffer<'q, D>,
        name: impl Into<String>,
    ) -> Result<IncompleteCommandBuffer<'q, D>> {
        if !self.measure_this_frame() {
            return Ok(cmd);
        }

        let cmd = cmd.write_timestamp(&mut self.timings, PipelineStage::ALL_COMMANDS)?;
        self.sections.insert(
            name.into(),
            SectionQuery {
                start_query: self.timings.current(),
                end_query: u32::MAX,
            },
        );

        Ok(cmd)
    }

    pub fn end_section<'q, D: ExecutionDomain>(
        &mut self,
        cmd: IncompleteCommandBuffer<'q, D>,
        name: &str,
    ) -> Result<IncompleteCommandBuffer<'q, D>> {
        if !self.measure_this_frame() {
            return Ok(cmd);
        }

        let cmd = cmd.write_timestamp(&mut self.timings, PipelineStage::ALL_COMMANDS)?;
        self.sections
            .get_mut(name)
            .ok_or(anyhow!("Section {name} not started."))?
            .end_query = self.timings.current();

        Ok(cmd)
    }

    pub fn new_frame(&mut self) {
        if self.frames_until_measure == 0 {
            self.frames_until_measure = self.interval;
            self.sections.clear();
            self.timings.reset();
            self.statistics.reset();
        } else {
            self.frames_until_measure -= 1;
        }

        // If enough frames have elapsed, poll results
        if self.frames_until_measure == self.interval - FRAMES_IN_FLIGHT as u32 - 1 {
            self.read_results().safe_unwrap();
        }
    }

    fn read_results(&mut self) -> Result<()> {
        let timestamps = self
            .timings
            .wait_for_results(0, (self.sections.len() * 2) as u32)?;
        for (name, queries) in self.sections.iter() {
            let start = *timestamps.get(queries.start_query as usize).unwrap();
            let end = *timestamps.get(queries.end_query as usize).unwrap();
            self.timing_results.insert(name.clone(), end - start);
        }
        Ok(())
    }

    fn measure_this_frame(&self) -> bool {
        self.frames_until_measure == self.interval
    }
}

pub trait TimedCommandBuffer {
    fn begin_section(
        self,
        timings: &mut RendererStatistics,
        name: impl Into<String>,
    ) -> Result<Self>
    where
        Self: Sized;
    fn end_section(self, timings: &mut RendererStatistics, name: &str) -> Result<Self>
    where
        Self: Sized;
}

impl<D: ExecutionDomain> TimedCommandBuffer for IncompleteCommandBuffer<'_, D> {
    fn begin_section(
        self,
        timings: &mut RendererStatistics,
        name: impl Into<String>,
    ) -> Result<Self>
    where
        Self: Sized, {
        timings.begin_section(self, name)
    }

    fn end_section(self, timings: &mut RendererStatistics, name: &str) -> Result<Self>
    where
        Self: Sized, {
        timings.end_section(self, name)
    }
}

pub trait StatisticsProvider {
    fn section_timings(&self) -> &HashMap<String, Duration>;
}

impl StatisticsProvider for &RendererStatistics {
    fn section_timings(&self) -> &HashMap<String, Duration> {
        &self.timing_results
    }
}

use super::component::BvGraphComponent;
use webgraph::prelude::*;

/// A decoder wrapper that perform a side effect on each decoded integer.
pub struct ConsumerDecoderFactory<F: SequentialDecoderFactory, C: Fn(BvGraphComponent, u64)> {
    factory: F,
    consumer: C,
}

impl<F, C> ConsumerDecoderFactory<F, C>
where
    F: SequentialDecoderFactory,
    C: Fn(BvGraphComponent, u64),
{
    pub fn new(factory: F, consumer: C) -> Self {
        Self { factory, consumer }
    }
}

impl<F, C> SequentialDecoderFactory for ConsumerDecoderFactory<F, C>
where
    F: SequentialDecoderFactory,
    C: Fn(BvGraphComponent, u64) + Clone,
{
    type Decoder<'a>
        = ConsumerDecoder<F::Decoder<'a>, C>
    where
        Self: 'a;

    #[inline(always)]
    fn new_decoder(&self) -> anyhow::Result<Self::Decoder<'_>> {
        Ok(ConsumerDecoder::new(
            self.factory.new_decoder()?,
            self.consumer.clone(),
        ))
    }
}

/// A wrapper over a generic [`Decode`] that keeps track of how much
/// bits each piece would take using different codes for compressions
pub struct ConsumerDecoder<D: Decode, C: Fn(BvGraphComponent, u64)> {
    codes_reader: D,
    consumer: C,
}

impl<D: Decode, C: Fn(BvGraphComponent, u64)> ConsumerDecoder<D, C> {
    /// Wrap a reader
    #[inline(always)]
    pub fn new(decoder: D, consumer: C) -> Self {
        Self {
            codes_reader: decoder,
            consumer,
        }
    }
}

impl<D: Decode, C: Fn(BvGraphComponent, u64)> Decode for ConsumerDecoder<D, C> {
    #[inline(always)]
    fn read_outdegree(&mut self) -> u64 {
        let decoded = self.codes_reader.read_outdegree();
        (self.consumer)(BvGraphComponent::Outdegree, decoded);
        decoded
    }

    #[inline(always)]
    fn read_reference_offset(&mut self) -> u64 {
        let decoded = self.codes_reader.read_reference_offset();
        (self.consumer)(BvGraphComponent::ReferenceOffset, decoded);
        decoded
    }

    #[inline(always)]
    fn read_block_count(&mut self) -> u64 {
        let decoded = self.codes_reader.read_block_count();
        (self.consumer)(BvGraphComponent::BlockCount, decoded);
        decoded
    }

    #[inline(always)]
    fn read_block(&mut self) -> u64 {
        let decoded = self.codes_reader.read_block();
        (self.consumer)(BvGraphComponent::Blocks, decoded);
        decoded
    }

    #[inline(always)]
    fn read_interval_count(&mut self) -> u64 {
        let decoded = self.codes_reader.read_interval_count();
        (self.consumer)(BvGraphComponent::IntervalCount, decoded);
        decoded
    }

    #[inline(always)]
    fn read_interval_start(&mut self) -> u64 {
        let decoded = self.codes_reader.read_interval_start();
        (self.consumer)(BvGraphComponent::IntervalStart, decoded);
        decoded
    }

    #[inline(always)]
    fn read_interval_len(&mut self) -> u64 {
        let decoded = self.codes_reader.read_interval_len();
        (self.consumer)(BvGraphComponent::IntervalLen, decoded);
        decoded
    }

    #[inline(always)]
    fn read_first_residual(&mut self) -> u64 {
        let decoded = self.codes_reader.read_first_residual();
        (self.consumer)(BvGraphComponent::FirstResidual, decoded);
        decoded
    }

    #[inline(always)]
    fn read_residual(&mut self) -> u64 {
        let decoded = self.codes_reader.read_residual();
        (self.consumer)(BvGraphComponent::Residual, decoded);
        decoded
    }
}

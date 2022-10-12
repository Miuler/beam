package org.apache.beam.sdk.io.azure.cosmos

import org.apache.beam.sdk.io.BoundedSource
import org.apache.beam.sdk.options.PipelineOptions

import java.util

class CosmosSource(readCosmos: ReadCosmos) extends BoundedSource {
  override def split(desiredBundleSizeBytes: Long, options: PipelineOptions): util.List[_ <: BoundedSource[Nothing]] =
    ???

  override def getEstimatedSizeBytes(options: PipelineOptions): Long = ???

  override def createReader(options: PipelineOptions): BoundedSource.BoundedReader[Nothing] = new CosmosBoundedReader()
}

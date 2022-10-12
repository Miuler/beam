package org.apache.beam.sdk.io.azure.cosmos

import org.apache.beam.sdk.io.BoundedSource

class CosmosBoundedReader extends BoundedSource.BoundedReader[String] {
  override def start(): Boolean = ???

  override def advance(): Boolean = ???

  override def getCurrent: String = ???

  override def getCurrentSource: String = ???

  override def close(): Unit = ???
}

package click.seichi.config

import cats.effect.Sync
import click.seichi.config.domain.ConfigLoader
import click.seichi.config.infrastructure.PropertiesConfigLoader

import java.util.Properties

object System {

  def getProperties[F[_]: Sync]: F[Properties] = {
    val configLoader: ConfigLoader[F, Properties] = new PropertiesConfigLoader[F]

    configLoader.load()
  }

}

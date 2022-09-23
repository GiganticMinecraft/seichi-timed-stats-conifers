package click.seichi.configuration

import cats.effect.Sync
import click.seichi.configuration.domain.{ConfigLoader, DatabaseConnectionInformation}
import click.seichi.configuration.infrastructure.PropertiesConfigLoader

import java.util.Properties

trait System[F[_]] {

  val api: ConfigurationAPI[F]

}

object System {

  import cats.implicits._

  def getProperties[F[_]: Sync]: F[System[F]] = {
    val configLoader: ConfigLoader[F, Properties] = new PropertiesConfigLoader[F]

    for {
      loadedProperties <- configLoader.load()
    } yield {
      new System[F] {
        override val api: ConfigurationAPI[F] = new ConfigurationAPI[F] {
          override def getDatabaseConnectionInformation: F[DatabaseConnectionInformation] =
            Sync[F].delay {
              val host = loadedProperties.getProperty("host")
              val password = loadedProperties.getProperty("password")
              val user = loadedProperties.getProperty("user")
              val port = loadedProperties.getProperty("post")
              DatabaseConnectionInformation(host, user, password, port.toInt)
            }
        }
      }
    }
  }

}

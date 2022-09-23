package click.seichi.configuration.domain

trait ConfigLoader[F[_], Config] {

  /**
   * @return configをロードしロードしたconfigを返す
   */
  def load(): F[Config]

}

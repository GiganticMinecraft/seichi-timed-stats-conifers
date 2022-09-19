package click.seichi.config.domain

trait ConfigLoader[F[_], Config] {

  /**
   * @return configをロードしロードしたconfigを返す
   */
  def load(): F[Config]

}

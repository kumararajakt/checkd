#include "databasemanager.h"

#include <QCoreApplication>
#include <QDateTime>
#include <QDebug>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QSqlDatabase>
#include <QSqlError>
#include <QSqlQuery>
#include <QStandardPaths>

#include "asyncsql/databaseconnection.h"

DatabaseManager &DatabaseManager::instance() {
  static DatabaseManager s_instance;
  return s_instance;
}

DatabaseManager::DatabaseManager(QObject *parent) : QObject(parent) {}

QString DatabaseManager::databasePath() const { return m_databasePath; }

QString DatabaseManager::resolveDatabasePath(const QString &customPath) const {
  if (!customPath.isEmpty()) {
    return customPath;
  }

  const QString defaultDir =
      QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);

  return QDir(defaultDir).filePath(QStringLiteral("checkd.db"));
}

bool DatabaseManager::initialize(const QString &customPath) {
  m_databasePath = resolveDatabasePath(customPath);

  QFileInfo fileInfo(m_databasePath);
  QDir dir = fileInfo.dir();
  if (!dir.exists()) {
    if (!dir.mkpath(QStringLiteral("."))) {
      qWarning() << "Failed to create database directory:" << dir.path();
      return false;
    }
  }

  if (!initSchema(m_databasePath)) {
    qWarning() << "Failed to initialize database schema at:" << m_databasePath;
    return false;
  }

  // Configure AsyncSql connection defaults
  AsyncSql::DatabaseConnection::setDefaultDriver(
      AsyncSql::DatabaseConnection::SQLite);
  AsyncSql::DatabaseConnection::setDefaultDatabaseName(m_databasePath);
  AsyncSql::DatabaseConnection::setDefaultConnectOptions(
      QStringLiteral("QSQLITE_BUSY_TIMEOUT=5000"));

  qInfo() << "CheckD database successfully initialized at:" << m_databasePath;
  return true;
}

bool DatabaseManager::initSchema(const QString &path) {
  const QString connName = QStringLiteral("schema_init_connection");
  {
    QSqlDatabase db =
        QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), connName);
    db.setDatabaseName(path);
    if (!db.open()) {
      qWarning() << "Cannot open database for schema initialization:"
                 << db.lastError().text();
      return false;
    }

    QSqlQuery q(db);
    if (!q.exec(QStringLiteral("PRAGMA foreign_keys = ON;"))) {
      qWarning() << "Failed to enable foreign keys:" << q.lastError().text();
    }

    if (!q.exec(QStringLiteral("PRAGMA journal_mode = WAL;"))) {
      qWarning() << "Failed to enable WAL mode:" << q.lastError().text();
    }

    const QString schema = QStringLiteral(
        "CREATE TABLE IF NOT EXISTS lists ("
        "    id         TEXT PRIMARY KEY,"
        "    name       TEXT NOT NULL,"
        "    color      TEXT NOT NULL,"
        "    position   INTEGER NOT NULL,"
        "    updated_at INTEGER NOT NULL DEFAULT 0"
        ");"
        "CREATE TABLE IF NOT EXISTS tasks ("
        "    id          TEXT PRIMARY KEY,"
        "    list_id     TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,"
        "    title       TEXT NOT NULL,"
        "    description TEXT NOT NULL DEFAULT '',"
        "    due_date    TEXT,"
        "    due_time    TEXT,"
        "    repeat_json TEXT,"
        "    position    INTEGER NOT NULL,"
        "    updated_at  INTEGER NOT NULL DEFAULT 0,"
        "    completed   INTEGER NOT NULL DEFAULT 0,"
        "    next_occurrence TEXT,"
        "    occurrence_index INTEGER NOT NULL DEFAULT 0,"
        "    notified_key TEXT"
        ");"
        "CREATE INDEX IF NOT EXISTS idx_tasks_list_id ON tasks(list_id);"
        "CREATE TABLE IF NOT EXISTS sync_links ("
        "    entity_type TEXT    NOT NULL,"
        "    entity_id   TEXT    NOT NULL,"
        "    parent_id   TEXT,"
        "    remote_id   TEXT,"
        "    dirty       INTEGER NOT NULL DEFAULT 1,"
        "    deleted     INTEGER NOT NULL DEFAULT 0,"
        "    updated_at  INTEGER NOT NULL DEFAULT 0,"
        "    synced_at   INTEGER NOT NULL DEFAULT 0,"
        "    PRIMARY KEY (entity_type, entity_id)"
        ");"
        "CREATE TABLE IF NOT EXISTS sync_state ("
        "    key   TEXT PRIMARY KEY,"
        "    value TEXT NOT NULL"
        ");");

    const QStringList statements =
        schema.split(QLatin1Char(';'), Qt::SkipEmptyParts);
    for (const QString &stmt : statements) {
      const QString trimmed = stmt.trimmed();
      if (!trimmed.isEmpty()) {
        if (!q.exec(trimmed)) {
          qWarning() << "Schema execution failed:" << trimmed << ":"
                     << q.lastError().text();
        }
      }
    }

    db.close();
  }
  QSqlDatabase::removeDatabase(connName);
  return true;
}

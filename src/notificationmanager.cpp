#include "notificationmanager.h"

#include <QDateTime>
#include <QDebug>
#include <QDir>
#include <QFile>
#include <QIcon>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QStandardPaths>
#include <QWindow>
#include <QSqlDatabase>
#include <QSqlError>
#include <QSqlQuery>

#include <KLocalizedString>
#include <KNotification>

#include "databasemanager.h"

NotificationManager &NotificationManager::instance()
{
    static NotificationManager s_instance;
    return s_instance;
}

NotificationManager::NotificationManager(QObject *parent)
    : QObject(parent)
{
    ensureNotifyRcExists();
    connect(&m_timer, &QTimer::timeout, this, &NotificationManager::onCheckTimer);
    // Check every 30 seconds
    m_timer.start(30000);
}

NotificationManager::~NotificationManager() = default;

void NotificationManager::setAssociatedWindow(QWindow *window)
{
    m_window = window;
}

QWindow *NotificationManager::associatedWindow() const
{
    return m_window;
}

void NotificationManager::onCheckTimer()
{
    checkNotifications();
}

QString NotificationManager::computeNextOccurrence(const QString &repeatJson, const QDate &fromDate)
{
    if (repeatJson.trimmed().isEmpty()) {
        return QString();
    }

    QJsonDocument doc = QJsonDocument::fromJson(repeatJson.toUtf8());
    if (!doc.isObject()) {
        return QString();
    }

    QJsonObject obj = doc.object();
    QString type = obj.value(QStringLiteral("type")).toString();
    if (type.isEmpty()) {
        type = obj.value(QStringLiteral("unit")).toString();
    }
    int interval = obj.value(QStringLiteral("interval")).toInt(1);
    if (interval < 1) {
        interval = 1;
    }

    QDate baseDate = fromDate.isValid() ? fromDate : QDate::currentDate();
    QDate nextDate = baseDate;

    if (type == QLatin1String("day")) {
        nextDate = baseDate.addDays(interval);
    } else if (type == QLatin1String("week")) {
        QJsonArray daysArr = obj.value(QStringLiteral("days")).toArray();
        if (daysArr.isEmpty()) {
            daysArr = obj.value(QStringLiteral("weekdays")).toArray();
        }

        if (!daysArr.isEmpty()) {
            QList<int> targetDays;
            for (const QJsonValue &v : daysArr) {
                targetDays << v.toInt();
            }

            bool found = false;
            for (int offset = 1; offset <= 7 * interval; ++offset) {
                QDate test = baseDate.addDays(offset);
                int jsDay = (test.dayOfWeek() % 7); // Mon=1..Sat=6, Sun=0
                if (targetDays.contains(jsDay)) {
                    nextDate = test;
                    found = true;
                    break;
                }
            }
            if (!found) {
                nextDate = baseDate.addDays(7 * interval);
            }
        } else {
            nextDate = baseDate.addDays(7 * interval);
        }
    } else if (type == QLatin1String("month")) {
        nextDate = baseDate.addMonths(interval);
    } else if (type == QLatin1String("year")) {
        nextDate = baseDate.addYears(interval);
    } else {
        nextDate = baseDate.addDays(1);
    }

    return nextDate.toString(QStringLiteral("yyyy-MM-dd"));
}

void NotificationManager::updateNotifiedKey(const QString &taskId, const QString &notifiedKey, qint64 nowMs)
{
    const QString dbPath = DatabaseManager::instance().databasePath();
    const QString connName = QStringLiteral("notif_key_update_conn");
    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), connName);
        db.setDatabaseName(dbPath);
        if (db.open()) {
            QSqlQuery u(db);
            u.prepare(QStringLiteral("UPDATE tasks SET notified_key = :key, updated_at = :now WHERE id = :id"));
            u.bindValue(QStringLiteral(":key"), notifiedKey);
            u.bindValue(QStringLiteral(":now"), nowMs);
            u.bindValue(QStringLiteral(":id"), taskId);
            u.exec();
            db.close();
        }
    }
    QSqlDatabase::removeDatabase(connName);
}

void NotificationManager::updateNextOccurrence(const QString &taskId, const QString &nextOcc, qint64 nowMs)
{
    const QString dbPath = DatabaseManager::instance().databasePath();
    const QString connName = QStringLiteral("notif_nextocc_conn");
    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), connName);
        db.setDatabaseName(dbPath);
        if (db.open()) {
            QSqlQuery u(db);
            u.prepare(QStringLiteral("UPDATE tasks SET next_occurrence = :nextOcc, updated_at = :now WHERE id = :id"));
            u.bindValue(QStringLiteral(":nextOcc"), nextOcc);
            u.bindValue(QStringLiteral(":now"), nowMs);
            u.bindValue(QStringLiteral(":id"), taskId);
            u.exec();
            db.close();
        }
    }
    QSqlDatabase::removeDatabase(connName);
}

void NotificationManager::ensureNotifyRcExists()
{
    const QString relativePath = QStringLiteral("knotifications6/checkd.notifyrc");
    if (!QStandardPaths::locate(QStandardPaths::GenericDataLocation, relativePath).isEmpty()) {
        return;
    }

    const QString targetDir = QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation) + QStringLiteral("/knotifications6");
    QDir().mkpath(targetDir);
    const QString targetFile = targetDir + QStringLiteral("/checkd.notifyrc");

    QFile file(targetFile);
    if (file.open(QIODevice::WriteOnly | QIODevice::Truncate | QIODevice::Text)) {
        static const char notifyRcContent[] =
            "[Global]\n"
            "IconName=checkd\n"
            "DesktopEntry=checkd\n"
            "Name=CheckD\n"
            "Comment=CheckD task manager\n"
            "\n"
            "[Event/taskNotification]\n"
            "Name=Task Reminder\n"
            "Comment=A reminder for a due or repeating task\n"
            "Action=Popup\n"
            "Urgency=Normal\n";
        file.write(notifyRcContent);
        file.close();
    }
}

void NotificationManager::sendNotification(const QString &taskId, const QString &listId,
                                           const QString &title, const QString &body,
                                           const QString &notifiedKey, bool isRepeating,
                                           const QString &repeatJson, const QString &nextOcc)
{
    Q_UNUSED(listId)
    Q_UNUSED(notifiedKey)
    Q_UNUSED(isRepeating)
    Q_UNUSED(repeatJson)
    Q_UNUSED(nextOcc)

    auto *notification = new KNotification(QStringLiteral("taskNotification"), KNotification::CloseOnTimeout);
    notification->setComponentName(QStringLiteral("checkd"));
    notification->setTitle(title.isEmpty() ? i18n("CheckD Task") : title);
    notification->setText(body);
    notification->setIconName(QIcon::hasThemeIcon(QStringLiteral("checkd")) ? QStringLiteral("checkd") : QStringLiteral("checkbox"));

    KNotificationAction *defaultAction = notification->addDefaultAction(i18n("Open CheckD"));
    connect(defaultAction, &KNotificationAction::activated, this, [this]() {
        if (m_window) {
            m_window->show();
            m_window->raise();
            m_window->requestActivate();
        }
    });

    KNotificationAction *completeAction = notification->addAction(i18n("Mark as Completed"));
    connect(completeAction, &KNotificationAction::activated, this, [this, taskId]() {
        markTaskCompleted(taskId);
    });

    notification->sendEvent();
}

void NotificationManager::markTaskCompleted(const QString &taskId)
{
    const QString dbPath = DatabaseManager::instance().databasePath();
    const QString connName = QStringLiteral("notif_complete_conn");
    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), connName);
        db.setDatabaseName(dbPath);
        if (db.open()) {
            QSqlQuery q(db);
            q.prepare(QStringLiteral("SELECT list_id, repeat_json, next_occurrence, occurrence_index FROM tasks WHERE id = :id"));
            q.bindValue(QStringLiteral(":id"), taskId);
            if (q.exec() && q.next()) {
                const QString listId = q.value(0).toString();
                const QString repeatJson = q.value(1).toString();
                const QString nextOcc = q.value(2).toString();
                const qint64 occIdx = q.value(3).toLongLong();
                const qint64 now = QDateTime::currentMSecsSinceEpoch();

                if (!repeatJson.trimmed().isEmpty()) {
                    QDate curDate = QDate::fromString(nextOcc, QStringLiteral("yyyy-MM-dd"));
                    if (!curDate.isValid()) {
                        curDate = QDate::currentDate();
                    }
                    QString newNextOcc = computeNextOccurrence(repeatJson, curDate);

                    QSqlQuery u(db);
                    u.prepare(QStringLiteral("UPDATE tasks SET next_occurrence = :nextOcc, occurrence_index = :idx, notified_key = NULL, updated_at = :now WHERE id = :id"));
                    u.bindValue(QStringLiteral(":nextOcc"), newNextOcc);
                    u.bindValue(QStringLiteral(":idx"), occIdx + 1);
                    u.bindValue(QStringLiteral(":now"), now);
                    u.bindValue(QStringLiteral(":id"), taskId);
                    u.exec();
                } else {
                    QSqlQuery u(db);
                    u.prepare(QStringLiteral("UPDATE tasks SET completed = 1, updated_at = :now WHERE id = :id"));
                    u.bindValue(QStringLiteral(":now"), now);
                    u.bindValue(QStringLiteral(":id"), taskId);
                    u.exec();
                }

                QSqlQuery s(db);
                s.prepare(QStringLiteral("INSERT OR REPLACE INTO sync_links (entity_type, entity_id, parent_id, dirty, updated_at) VALUES ('task', :id, :listId, 1, :now)"));
                s.bindValue(QStringLiteral(":id"), taskId);
                s.bindValue(QStringLiteral(":listId"), listId);
                s.bindValue(QStringLiteral(":now"), now);
                s.exec();
            }
            db.close();
        }
    }
    QSqlDatabase::removeDatabase(connName);

    Q_EMIT taskStateChanged();
}

void NotificationManager::resetNotifications()
{
    const QString dbPath = DatabaseManager::instance().databasePath();
    if (dbPath.isEmpty()) {
        return;
    }

    const QString connName = QStringLiteral("notif_reset_conn");
    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), connName);
        db.setDatabaseName(dbPath);
        if (db.open()) {
            QSqlQuery q(db);
            q.exec(QStringLiteral("UPDATE tasks SET notified_key = NULL"));
            db.close();
        }
    }
    QSqlDatabase::removeDatabase(connName);
}

void NotificationManager::checkNotifications()
{
    const QString dbPath = DatabaseManager::instance().databasePath();
    if (dbPath.isEmpty()) {
        return;
    }

    const QString connName = QStringLiteral("notif_check_conn");
    struct TaskNotif {
        QString id;
        QString listId;
        QString listName;
        QString title;
        QString description;
        QString dueDate;
        QString dueTime;
        QString repeatJson;
        QString nextOcc;
        QString notifiedKey;
    };
    QList<TaskNotif> pending;

    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), connName);
        db.setDatabaseName(dbPath);
        if (db.open()) {
            QSqlQuery q(db);
            if (q.exec(QStringLiteral("SELECT t.id, t.list_id, COALESCE(l.name, ''), t.title, t.description, t.due_date, t.due_time, t.repeat_json, t.next_occurrence, t.notified_key "
                                      "FROM tasks t LEFT JOIN lists l ON l.id = t.list_id WHERE t.completed = 0"))) {
                while (q.next()) {
                    TaskNotif t;
                    t.id = q.value(0).toString();
                    t.listId = q.value(1).toString();
                    t.listName = q.value(2).toString();
                    t.title = q.value(3).toString();
                    t.description = q.value(4).toString();
                    t.dueDate = q.value(5).toString();
                    t.dueTime = q.value(6).toString();
                    t.repeatJson = q.value(7).toString();
                    t.nextOcc = q.value(8).toString();
                    t.notifiedKey = q.value(9).toString();
                    pending.append(t);
                }
            }
            db.close();
        }
    }
    QSqlDatabase::removeDatabase(connName);

    const QDateTime currentDateTime = QDateTime::currentDateTime();
    const QDate currentDate = currentDateTime.date();
    const qint64 nowMs = currentDateTime.toMSecsSinceEpoch();

    for (const TaskNotif &t : std::as_const(pending)) {
        // 1. Due date check
        if (!t.dueDate.trimmed().isEmpty()) {
            QDate dueDate = QDate::fromString(t.dueDate, QStringLiteral("yyyy-MM-dd"));
            if (dueDate.isValid()) {
                QDateTime dueDateTime;
                if (!t.dueTime.trimmed().isEmpty()) {
                    QTime dueTime = QTime::fromString(t.dueTime, QStringLiteral("hh:mm"));
                    if (!dueTime.isValid()) {
                        dueTime = QTime::fromString(t.dueTime, QStringLiteral("h:mm"));
                    }
                    dueDateTime = QDateTime(dueDate, dueTime.isValid() ? dueTime : QTime(0, 0, 0));
                } else {
                    dueDateTime = QDateTime(dueDate, QTime(0, 0, 0));
                }

                if (currentDateTime >= dueDateTime) {
                    const QString key = QStringLiteral("due:") + t.dueDate + (t.dueTime.isEmpty() ? QString() : (QLatin1Char('T') + t.dueTime));
                    if (t.notifiedKey != key) {
                        const bool isOverdue = t.dueTime.isEmpty() ? (dueDate < currentDate) : (currentDateTime > dueDateTime);
                        QString when;
                        if (!t.dueTime.isEmpty()) {
                            when = QStringLiteral("%1 · %2").arg(dueDate.toString(QStringLiteral("MMM d, yyyy")), t.dueTime);
                        } else {
                            when = dueDate.toString(QStringLiteral("MMM d, yyyy"));
                        }

                        QString body;
                        if (isOverdue) {
                            if (!t.listName.isEmpty()) {
                                body = QStringLiteral("Overdue · %1 · %2").arg(t.listName, when);
                            } else {
                                body = QStringLiteral("Overdue · %1").arg(when);
                            }
                        } else {
                            if (!t.listName.isEmpty()) {
                                body = QStringLiteral("%1 · %2").arg(t.listName, when);
                            } else {
                                body = when;
                            }
                        }
                        if (!t.description.trimmed().isEmpty()) {
                            body += QStringLiteral("\n") + t.description.trimmed();
                        }

                        sendNotification(t.id, t.listId, t.title, body, key, false, QString(), QString());
                        updateNotifiedKey(t.id, key, nowMs);
                    }
                }
            }
        }

        // 2. Repeating task check
        if (!t.repeatJson.trimmed().isEmpty()) {
            QString nextOcc = t.nextOcc;
            if (nextOcc.isEmpty()) {
                QJsonDocument doc = QJsonDocument::fromJson(t.repeatJson.toUtf8());
                QString startStr = doc.object().value(QStringLiteral("startDate")).toString();
                if (startStr.isEmpty()) {
                    startStr = doc.object().value(QStringLiteral("start")).toString();
                }
                QDate startDate = QDate::fromString(startStr, QStringLiteral("yyyy-MM-dd"));
                if (!startDate.isValid()) {
                    startDate = currentDate;
                }
                QDate occ = startDate;
                while (occ < currentDate) {
                    QDate adv = QDate::fromString(computeNextOccurrence(t.repeatJson, occ), QStringLiteral("yyyy-MM-dd"));
                    if (!adv.isValid() || adv <= occ) {
                        break;
                    }
                    occ = adv;
                }
                nextOcc = occ.toString(QStringLiteral("yyyy-MM-dd"));
                updateNextOccurrence(t.id, nextOcc, nowMs);
            }

            QDate occDate = QDate::fromString(nextOcc, QStringLiteral("yyyy-MM-dd"));
            if (occDate.isValid()) {
                QJsonDocument doc = QJsonDocument::fromJson(t.repeatJson.toUtf8());
                QString timeStr = doc.object().value(QStringLiteral("time")).toString();
                QDateTime occDateTime;
                if (!timeStr.isEmpty()) {
                    QTime occTime = QTime::fromString(timeStr, QStringLiteral("hh:mm"));
                    occDateTime = QDateTime(occDate, occTime.isValid() ? occTime : QTime(0, 0, 0));
                } else {
                    occDateTime = QDateTime(occDate, QTime(0, 0, 0));
                }

                if (currentDateTime >= occDateTime) {
                    const QString key = QStringLiteral("occ:") + nextOcc + (timeStr.isEmpty() ? QString() : (QLatin1Char('T') + timeStr));
                    if (t.notifiedKey != key) {
                        const bool isOverdue = timeStr.isEmpty() ? (occDate < currentDate) : (currentDateTime > occDateTime);
                        QString when;
                        if (!timeStr.isEmpty()) {
                            when = QStringLiteral("%1 · %2").arg(occDate.toString(QStringLiteral("MMM d, yyyy")), timeStr);
                        } else {
                            when = occDate.toString(QStringLiteral("MMM d, yyyy"));
                        }

                        QString body;
                        if (isOverdue) {
                            if (!t.listName.isEmpty()) {
                                body = QStringLiteral("Overdue · %1 · %2").arg(t.listName, when);
                            } else {
                                body = QStringLiteral("Overdue · %1").arg(when);
                            }
                        } else {
                            if (!t.listName.isEmpty()) {
                                body = QStringLiteral("%1 · %2").arg(t.listName, when);
                            } else {
                                body = when;
                            }
                        }
                        if (!t.description.trimmed().isEmpty()) {
                            body += QStringLiteral("\n") + t.description.trimmed();
                        }

                        sendNotification(t.id, t.listId, t.title, body, key, true, t.repeatJson, nextOcc);
                        updateNotifiedKey(t.id, key, nowMs);
                    }
                }
            }
        }
    }
}

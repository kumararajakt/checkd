#ifndef APPSETTINGS_H
#define APPSETTINGS_H

#include <QObject>
#include <QSettings>

class AppSettings : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool closeToTray READ closeToTray WRITE setCloseToTray NOTIFY closeToTrayChanged)
    Q_PROPERTY(bool autostart READ autostart WRITE setAutostart NOTIFY autostartChanged)
    Q_PROPERTY(QString lastSelectedListId READ lastSelectedListId WRITE setLastSelectedListId NOTIFY lastSelectedListIdChanged)

public:
    static AppSettings &instance();

    explicit AppSettings(QObject *parent = nullptr);
    ~AppSettings() override = default;

    bool closeToTray() const;
    void setCloseToTray(bool enabled);

    bool autostart() const;
    void setAutostart(bool enabled);

    QString lastSelectedListId() const;
    void setLastSelectedListId(const QString &id);

Q_SIGNALS:
    void closeToTrayChanged();
    void autostartChanged();
    void lastSelectedListIdChanged();

private:
    QString autostartFilePath() const;
    void writeAutostartFile(bool enabled);

    QSettings m_settings;
};

#endif // APPSETTINGS_H

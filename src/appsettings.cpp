#include "appsettings.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QStandardPaths>
#include <QTextStream>

AppSettings &AppSettings::instance()
{
    static AppSettings s_instance;
    return s_instance;
}

AppSettings::AppSettings(QObject *parent)
    : QObject(parent)
{
}

bool AppSettings::closeToTray() const
{
    return m_settings.value(QStringLiteral("general/closeToTray"), true).toBool();
}

void AppSettings::setCloseToTray(bool enabled)
{
    if (closeToTray() == enabled) {
        return;
    }
    m_settings.setValue(QStringLiteral("general/closeToTray"), enabled);
    m_settings.sync();
    Q_EMIT closeToTrayChanged();
}

QString AppSettings::autostartFilePath() const
{
    const QString configDir = QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation);
    return QDir(configDir).filePath(QStringLiteral("autostart/checkd.desktop"));
}

bool AppSettings::autostart() const
{
    const QString filePath = autostartFilePath();
    if (QFile::exists(filePath)) {
        return true;
    }
    const QString configDir = QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation);
    const QString altPath = QDir(configDir).filePath(QStringLiteral("autostart/com.kumar.checkd.desktop"));
    if (QFile::exists(altPath)) {
        return true;
    }
    return m_settings.value(QStringLiteral("general/autostart"), false).toBool();
}

void AppSettings::setAutostart(bool enabled)
{
    if (autostart() == enabled) {
        return;
    }

    writeAutostartFile(enabled);
    m_settings.setValue(QStringLiteral("general/autostart"), enabled);
    m_settings.sync();
    Q_EMIT autostartChanged();
}

void AppSettings::writeAutostartFile(bool enabled)
{
    const QString filePath = autostartFilePath();
    const QString configDir = QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation);
    const QString altPath = QDir(configDir).filePath(QStringLiteral("autostart/com.kumar.checkd.desktop"));

    if (enabled) {
        const QFileInfo fileInfo(filePath);
        const QDir dir = fileInfo.dir();
        if (!dir.exists()) {
            dir.mkpath(QStringLiteral("."));
        }

        QString execPath = QStandardPaths::findExecutable(QStringLiteral("checkd"));
        if (execPath.isEmpty()) {
            execPath = QCoreApplication::applicationFilePath();
        } else {
            execPath = QStringLiteral("checkd");
        }

        QFile file(filePath);
        if (file.open(QIODevice::WriteOnly | QIODevice::Truncate | QIODevice::Text)) {
            QTextStream out(&file);
            out << "[Desktop Entry]\n";
            out << "Type=Application\n";
            out << "Name=CheckD\n";
            out << "Comment=Task & Todo Manager\n";
            out << "Exec=" << execPath << "\n";
            out << "Icon=checkd\n";
            out << "Terminal=false\n";
            out << "Categories=Utility;\n";
            out << "X-GNOME-Autostart-enabled=true\n";
            file.close();
        }
    } else {
        if (QFile::exists(filePath)) {
            QFile::remove(filePath);
        }
        if (QFile::exists(altPath)) {
            QFile::remove(altPath);
        }
    }
}

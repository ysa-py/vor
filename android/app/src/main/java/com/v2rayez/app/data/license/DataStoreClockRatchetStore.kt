package com.v2rayez.app.data.license

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.longPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.flow.first
import javax.inject.Inject
import javax.inject.Singleton

private val Context.clockDataStore: DataStore<Preferences> by preferencesDataStore(name = "vor_clock")

/**
 * DataStore-backed monotonic ratchet persistence for [LicenseClockCore].
 *
 * Kept in its own preferences file (`vor_clock`) so a ratchet write can never
 * race the license-token store (`vor_license`) used by [LicenseRepository].
 * Reads/writes are suspend and failure-tolerant at the caller.
 */
@Singleton
class DataStoreClockRatchetStore @Inject constructor(
    @ApplicationContext private val context: Context,
) : ClockRatchetStore {

    override suspend fun read(): Long =
        context.clockDataStore.data.first()[KEY_RATCHET] ?: 0L

    override suspend fun write(seconds: Long) {
        context.clockDataStore.edit { preferences ->
            val current = preferences[KEY_RATCHET] ?: 0L
            if (seconds > current) preferences[KEY_RATCHET] = seconds
        }
    }

    companion object {
        private val KEY_RATCHET = longPreferencesKey("vor_clock_ratchet_epoch_seconds")
    }
}

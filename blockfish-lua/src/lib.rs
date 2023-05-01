use mlua::prelude::*;

struct Service(blockfish::ai::AI);
impl LuaUserData for Service {
  fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
    fields.add_field_method_set("config", |_, this, msg: Config| {
      this.0.config_mut().search_limit = msg.0.search_limit;
      this.0.config_mut().parameters.row_factor = msg.0.parameters.row_factor;
      this.0.config_mut().parameters.piece_estimate_factor = msg.0.parameters.piece_estimate_factor;
      this.0.config_mut().parameters.i_dependency_factor = msg.0.parameters.i_dependency_factor;
      this.0.config_mut().parameters.piece_penalty = msg.0.parameters.piece_penalty;
      Ok(())
    });
  }
  fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
    methods.add_method_mut("analyze", |_, this, msg: Snapshot| {
      let mut analysis = this.0.analyze(msg.0);
      analysis.wait();
      let mut move_ids = analysis.all_moves().collect::<Vec<_>>();
      move_ids.sort_by(|&m, &n| analysis.cmp(m, n));
      let stats = analysis.stats().map(Stats);
      let res = move_ids
        .iter()
        .map(|&id| Suggestion(analysis.suggestion(id, std::usize::MAX)))
        .collect::<Vec<_>>();
      Ok((stats, res))
    });
    methods.add_method("config", |_, this, ()| {
      Ok(Config(this.0.config().clone()))
    });
  }
}

struct Stats(blockfish::ai::Stats);
impl<'lua> ToLua<'lua> for Stats {
  fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
    let table = lua.create_table()?;
    table.set("iterations", self.0.iterations)?;
    table.set("nodes", self.0.nodes)?;
    table.set("time_taken_ms", self.0.time_taken.as_millis())?;
    Ok(LuaValue::Table(table))
  }
}

struct Suggestion(blockfish::ai::Suggestion);
impl<'lua> ToLua<'lua> for Suggestion {
  fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
    let table = lua.create_table()?;
    table.set("rating", self.0.rating)?;
    let inputs = lua.create_table()?;
    for (i, &input) in self.0.inputs.iter().enumerate() {
      inputs.set(i + 1, match input {
        blockfish::Input::Left => 1,
        blockfish::Input::Right => 2,
        blockfish::Input::CW => 3,
        blockfish::Input::CCW => 4,
        blockfish::Input::Flip => 5,
        blockfish::Input::Hold => 8,
        blockfish::Input::SD => 7,
        blockfish::Input::HD => 6,
      })?;
    }
    table.set("inputs", inputs)?;
    Ok(LuaValue::Table(table))
  }
}

struct Snapshot(blockfish::ai::Snapshot);
impl<'lua> FromLua<'lua> for Snapshot {
  fn from_lua(lua_value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
    let table = match lua_value {
      LuaValue::Table(table) => table,
      _ => return Err(LuaError::FromLuaConversionError {
        from: "LuaValue",
        to: "Snapshot",
        message: Some("expected table".to_string()),
      }),
    };
    fn color(ch: char) -> Option<blockfish::Color> {
      blockfish::Color::try_from(ch).ok()
    }
    let hold: String = table.get("hold")?;
    let next: String = table.get("next")?;
    let field: Vec<bool> = table.get("field")?;
    let mut matrix = blockfish::BasicMatrix::with_cols(10);
    for i in 0..field.len().clamp(0, 400) {
      if field[i] {
        matrix.set(((i / 10) as u16, (i % 10) as u16));
      }
    }
    Ok(Snapshot(blockfish::ai::Snapshot {
      hold: hold.chars().next().and_then(color),
      queue: next.chars().filter_map(color).collect(),
      matrix,
    }))
  }
}

#[derive(Clone)]
struct Config(blockfish::Config);
impl LuaUserData for Config {
  fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
    // setters
    fields.add_field_method_set("search_limit", |_, this, value: usize| {
      this.0.search_limit = value;
      Ok(())
    });
    fields.add_field_method_set("row_factor", |_, this, value: i64| {
      this.0.parameters.row_factor = value;
      Ok(())
    });
    fields.add_field_method_set("piece_estimate_factor", |_, this, value: i64| {
      this.0.parameters.piece_estimate_factor = value;
      Ok(())
    });
    fields.add_field_method_set("i_dependency_factor", |_, this, value: i64| {
      this.0.parameters.i_dependency_factor = value;
      Ok(())
    });
    fields.add_field_method_set("piece_penalty", |_, this, value: i64| {
      this.0.parameters.piece_penalty = value;
      Ok(())
    });

    // getters
    fields.add_field_method_get("search_limit", |_, this| {
      Ok(this.0.search_limit)
    });
    fields.add_field_method_get("row_factor", |_, this| {
      Ok(this.0.parameters.row_factor)
    });
    fields.add_field_method_get("piece_estimate_factor", |_, this| {
      Ok(this.0.parameters.piece_estimate_factor)
    });
    fields.add_field_method_get("i_dependency_factor", |_, this| {
      Ok(this.0.parameters.i_dependency_factor)
    });
    fields.add_field_method_get("piece_penalty", |_, this| {
      Ok(this.0.parameters.piece_penalty)
    });
  }
}

#[derive(Clone)]
struct RotationSystem(blockfish::ShapeTable);
impl LuaUserData for RotationSystem {
}

#[mlua::lua_module]
fn blockfish(lua: &Lua) -> LuaResult<LuaTable> {
  let exports = lua.create_table()?;
  exports.set("init", lua.create_function(|_, (config, rotation_system): (Config, RotationSystem)| {
    Ok(Service(blockfish::ai::AI::new_with_shapetable(config.0, rotation_system.0)))
  })?)?;
  exports.set("default_config", lua.create_function(|_, ()| {
    Ok(Config(blockfish::Config::default()))
  })?)?;
  exports.set("new_rs", lua.create_function(|_, rotation_system: String| {
    let ruleset = serde_json::from_str::<block_stacker::Ruleset>(&rotation_system).expect("invalid ruleset");
    Ok(RotationSystem(blockfish::ShapeTable::from_ruleset(&ruleset)))
  })?)?;
  Ok(exports)
}

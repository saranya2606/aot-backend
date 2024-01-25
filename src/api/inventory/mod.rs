use super::{auth::session::AuthUser, error, PgPool};
use actix_web::{error::ErrorBadRequest, web, Responder, Result};
use self::util::{remove_game, NewAttack};
use crate::api;
use crate::models::{AttackerType, BuildingType, MineType, DefenderType};
use actix_web::error::ErrorBadRequest;
use actix_web::{web, HttpResponse, Responder, Result};
use serde::{Deserialize, Serialize};

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/inventory").route(web::get().to(get_inventory)))
        .service(web::resource("/upgrade").route(web::post().to(upgrade_block)));
}



#[derive(Deserialize, Serialize)]
pub struct UpgradeSuccessResponse {
    message: String,
    block_level: i32,
    block_hp: i32,  
    artifacts_stored: i32,  
}


#[derive(Debug, Serialize, Deserialize)]
struct InventoryItem {
    item_id: i32,
    item_level: i32,
    upgrade_cost: i32,
    current_damage: Option<i32>,
    current_hp: Option<i32>,
    current_artifacts: Option<i32>,
    current_radius: Option<i32>,
    current_speed: Option<i32>,
    upgrade_damage: Option<i32>,
    upgrade_hp: Option<i32>,
    upgrade_artifacts: Option<i32>,
    upgrade_radius: Option<i32>,
    upgrade_speed: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InventoryCategory {
    category: String,
    items: Vec<InventoryItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InventoryResponse {
    categories: Vec<InventoryCategory>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct UpgradeRequest {
    pub block_type: String,
    pub variant: i32,
}


async fn upgrade_block(upgrade_data: Json<UpgradeRequest>, pool: Data<PgPool>, user: AuthUser) -> Result<impl Responder> {
    let user_id = user.0;
    use crate::schema::{available_blocks, attacker_type, defender_type, mine_type, building_type, user};
    
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    
    let block_type = upgrade_data.block_type;
    let variant = upgrade_data.variant;
    if(block_type == "attacker"){
        let level = attacker_type::table
        .find(variant)
        .first::<AttackerType>(conn)
        .map_err(|err| DieselError {
            table: "attacker_type",
            function: function!(),
            error: err,
        })?
        .level;

        let hp = attacker_type::table
        .find(variant)
        .first::<AttackerType>(conn)
        .map_err(|err| DieselError {
            table: "attacker_type",
            function: function!(),
            error: err,
        })?
        .hp;

    }else{
        let level = building_type::table
        .find(variant)
        .first::<BuildingType>(conn)
        .map_err(|err| DieselError {
            table: "building_type",
            function: function!(),
            error: err,
        })?
        .level;

        let hp = 0;

    }
    

    let upgrade_cost = match block_type {
        "attacker" => attacker_type::table
            .find(variant)
            .first::<AttackerType>(conn)
            .map_err(|err| DieselError {
                table: "attacker_type",
                function: function!(),
                error: err,
            })?
            .cost,
    
        "defender" => defender_type::table
            .find(variant)
            .first::<DefenderType>(conn)
            .map_err(|err| DieselError {
                table: "defender_type",
                function: function!(),
                error: err,
            })?
            .cost,
    
        "mine" => mine_type::table
            .find(variant)
            .first::<MineType>(conn)
            .map_err(|err| DieselError {
                table: "mine_type",
                function: function!(),
                error: err,
            })?
            .cost,
    
        "building" | "bank" => building_type::table
            .find(variant)
            .first::<BuildingType>(conn)
            .map_err(|err| DieselError {
                table: "building_type",
                function: function!(),
                error: err,
            })?
            .cost,
    
        _ => return Err(ErrorNotFound("Block not found or cannot be upgraded")),
    };

    if user.artifacts < upgrade_cost {
        return Err(ErrorBadRequest("Block not found or cannot be upgraded"));
    }
    

    deduct_user_artifacts(user_id, upgrade_cost, &conn)?;

    if block_type == "attacker"{
        update_available_blocks_attacker(user_id, variant, &conn);
    }
    else{
        update_available_blocks(user_id, variant, &conn)?;

    }
    

let response = UpgradeSuccessResponse {
        message: "Upgrade success",
        block_level: level, 
        block_hp: hp,  
        artifacts_stored: user.artifacts,
    };

    Ok(HttpResponse::Ok().json(response))
}




async fn get_inventory(pool: web::Data<PgPool>, user: AuthUser) -> Result<impl Responder> {
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    let user_id = user.0; 

    if let Ok(user_blocks) = available_blocks::table
        .filter(available_blocks::user_id.eq(user_id))
        .load::<AvailableBlocks>(&conn)
        .map_err(|err| DieselError {
            table: "available_blocks",
            function: function!(),
            error: err,
        })?
    {
        let mut categories = Vec::new();

        for user_block in user_blocks {
            if let Ok(block) = block_type::table
                .filter(block_type::id.eq(user_block.block_type_id))
                .first::<BlockType>(&conn)
                .map_err(|err| DieselError {
                    table: "block_type",
                    function: function!(),
                    error: err,
                })?;

            {
                let mut items = Vec::new();
                let category = block.category;
                
                let response =  match category {
                    "building" | "bank"=> {
                        let building = building_type::table
                        .filter(building_type::id.eq(block.building_type))
                        .first::<BuildingType>(&conn)
                        InventoryItem{
                            item_id: building.id,
                            item_level: building.level,
                            upgrade_cost:get_next_level(block.id, &conn)
                            .map(|nlr| nlr.cost)
                            .unwrap_or_default(),
                            current_damage: 0,
                            current_hp: 0,
                            current_artifacts: 0,
                            current_radius: 0,
                            current_speed: 0,
                            upgrade_damage: 0,
                            upgrade_hp: 0,
                            upgrade_artifacts: 0,
                            upgrade_radius: 0,
                            upgrade_speed: 0,



                        }
                    },
                    "mine" => {
                        let mine = mine_type::table
                        .filter(mine_type::id.eq(block.mine_type))
                        .first::<MineType>(&conn)
                        InventoryItem{
                            item_id: mine.id,
                            item_level: mine.level,
                            upgrade_cost: get_next_level(block.id, &conn)
                            .map(|nlr| nlr.cost)
                            .unwrap_or_default(),
                            current_damage: mine.damage,
                            current_hp: 0,
                            current_artifacts: 0,
                            current_radius: mine.radius,
                            current_speed: 0,
                            upgrade_damage: get_next_level(block.id, &conn)
                            .map(|nlr| nlr.damage)
                            .unwrap_or_default(), 
                            upgrade_hp: 0,
                            upgrade_artifacts: 0,
                            upgrade_radius: get_next_level(block.id, &conn)
                            .map(|nlr| nlr.radius)
                            .unwrap_or_default(), 
                            upgrade_speed: 0,

                        }

                    },
                    "defender" => {
                        let defender = defender_type::table
                        .filter(defender_type::id.eq(block.defender_type))
                        .first::<DefenderType>(&conn)
                        InventoryItem{
                            item_id: defender.id,
                            item_level: defender.level,
                            upgrade_cost: get_next_level(block.id, &conn)
                            .map(|nlr| nlr.cost)
                            .unwrap_or_default(),
                            current_damage: defender.damage,
                            current_hp: 0,
                            current_artifacts: 0,
                            current_radius: defender.radius,
                            current_speed: defender.speed,
                            upgrade_damage: get_next_level(block.id, &conn)
                                .map(|nlr| nlr.damage)
                                .unwrap_or_default(),
                            upgrade_hp: 0,
                            upgrade_artifacts: 0,
                            upgrade_radius: get_next_level(block.id, &conn)
                            .map(|nlr| nlr.radius)
                            .unwrap_or_default(),
                            upgrade_speed: get_next_level(block.id, &conn)
                            .map(|nlr| nlr.speed)
                            .unwrap_or_default(),
                        }


                    },

                    "attacker" => {
                        let attacker = attacker_type::table
                        .filter(attacker_type::id.eq(block.attacker_type))
                        .first::<AttackerType>(&conn).map_err(|err| DieselError {
                            table: "attacker_type",
                            function: function!(),
                            error: err,
                        })?;

                        InventoryItem{
                            item_id: attacker.id,
                            item_level: attacker.level,
                            upgrade_cost: get_next_level_attacker(block.id, &conn)
                            .map(|nlr| nlr.cost)
                            .unwrap_or_default(),
                            current_damage: attacker.damage,
                            current_hp: attacker.max_health,
                            current_artifacts: 0,
                            current_radius: attacker.radius,
                            current_speed: attacker.speed,
                            upgrade_damage: get_next_level_attacker(block.id, &conn)
                                .map(|nlr| nlr.damage)
                                .unwrap_or_default(),
                            upgrade_hp: get_next_level_attacker(block.id, &conn)
                                .map(|nlr| nlr.max_health)
                                .unwrap_or_default(),
                            upgrade_artifacts: 0,
                            upgrade_radius: get_next_level_attacker(block.id, &conn)
                            .map(|nlr| nlr.radius)
                            .unwrap_or_default(),
                            upgrade_speed: get_next_level_attacker(block.id, &conn)
                            .map(|nlr| nlr.speed)
                            .unwrap_or_default(),
                        }


                    },
                }
                items.push(response);

                categories.push(InventoryCategory {
                        category: category,
                        items: items,
                    });
                }
            }
        

        let response = InventoryResponse { categories };

        Ok(HttpResponse::Ok().json(response))
    } else {
        Err(ErrorNotFound("Blocks not found"))
    }
}



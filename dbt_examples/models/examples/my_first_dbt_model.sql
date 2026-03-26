{{
   config(
     materialized='table',
     schema='examples',
     alias='my_first_dbt_model'
   )
}}

with 
source_data as (
   select 1 as id
   union all
   select null as id
)

select * from source_data

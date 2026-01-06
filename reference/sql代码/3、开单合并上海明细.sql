DROP TABLE IF EXISTS report_bill_detail;
CREATE TABLE report_bill_detail (
    `开单门店` VARCHAR(255),
    `病历编号` VARCHAR(255),
    `顾客姓名` VARCHAR(255),
    `性别` VARCHAR(10),
    `客户状态` VARCHAR(50),
    `咨询师` VARCHAR(255),
    `项目类型` VARCHAR(255),
    `项目分类` VARCHAR(255),
    `项目ID` VARCHAR(255),
    `开单项目` VARCHAR(255),
    `客户来源` VARCHAR(255),
    `规格` VARCHAR(255),
    `数量` INT,
    `单位` VARCHAR(50),
    `支付方式` VARCHAR(255),
    `实收金额` DECIMAL(15, 2),
    `开单人` VARCHAR(255),
    `业绩人` VARCHAR(255),
    `权益人` VARCHAR(255),
    `付款时间` DATETIME,
    `最早开单时间` DATETIME,
    `最后消耗时间` DATETIME,
    `开单时间` DATETIME,
    `备注` TEXT,
    `实质消费` DECIMAL(15, 2),
    `促销活动` VARCHAR(255),
    `组合项目` VARCHAR(255),
    `是否是网络订单` BOOLEAN,
    `是否是促销项目` BOOLEAN,
    `客户类型` INT,
    `开单提点类型` VARCHAR(255),
    `订单来源` INT,
    `是否是内购订单` INT
);

-- Step 3: Insert data into the table
INSERT INTO report_bill_detail (
    `开单门店`, `病历编号`, `顾客姓名`, `性别`, `客户状态`, `咨询师`,
    `项目类型`, `项目分类`, `项目ID`,`开单项目`, `客户来源`, `规格`, `数量`, `单位`,
    `支付方式`, `实收金额`, `开单人`, `业绩人`, `权益人`, `付款时间`,
    `最早开单时间`, `最后消耗时间`, `开单时间`, `备注`, `实质消费`, 
    `促销活动`, `组合项目`, `是否是网络订单`, `是否是促销项目`, 
    `客户类型`, `开单提点类型`, `订单来源`,`是否是内购订单`
		)
    
    SELECT
    *
    FROM report_bill_detail_temp
    WHERE 支付方式 NOT LIKE "%科技转门诊%"
    
  UNION ALL
    
    SELECT
    *
    FROM report_bill_detail_sh